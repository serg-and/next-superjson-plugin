use std::ops::IndexMut;

use swc_core::{
    common::{util::take::Take, DUMMY_SP},
    ecma::{
        ast::*,
        utils::{prepend_stmt, ExprFactory},
        visit::*,
    },
};

use crate::{utils::*, Config};

static SSG_EXPORTS: &[&str; 2] = &["getStaticProps", "getServerSideProps"];
static INITIAL_PROPS: &str = "getInitialProps";

// import { withSuperJSONProps as _withSuperJSONProps } from "next-superjson-plugin/tools";
static SUPERJSON_PROPS_IMPORTED: &str = "withSuperJSONProps";
pub static SUPERJSON_PROPS_LOCAL: &str = "_withSuperJSONProps";

// import { withSuperJSONInitProps as _withSuperJSONInitProps } from "next-superjson-plugin/tools";
static SUPERJSON_INIT_PROPS_IMPORTED: &str = "withSuperJSONInitProps";
pub static SUPERJSON_INIT_PROPS_LOCAL: &str = "_withSuperJSONInitProps";

// import { withSuperJSONPage as _withSuperJSONPage } from "next-superjson-plugin/tools";
static SUPERJSON_PAGE_IMPORTED: &str = "withSuperJSONPage";
pub static SUPERJSON_PAGE_LOCAL: &str = "_withSuperJSONPage";

// import { not_gSSP as _NEXT_SUPERJSON_IMPORTED_PROPS } from '..'
// const  _NEXT_SUPERJSON_SSG_PROPS = wrap(_NEXT_SUPERJSON_IMPORTED_PROPS)
// export { _NEXT_SUPERJSON_SSG_PROPS as gSSP }
pub static NEXT_SSG_PROPS_LOCAL: &str = "_NEXT_SUPERJSON_IMPORTED_PROPS";
pub static NEXT_SSG_PROPS_ORIG: &str = "_NEXT_SUPERJSON_SSG_PROPS";

// import { unwrapped as _NEXT_SUPERJSON_IMPORTED_PAGE } from 'src'
// export default wrap(_NEXT_SUPERJSON_IMPORTED_PAGE)
static NEXT_PAGE_LOCAL: &str = "_NEXT_SUPERJSON_IMPORTED_PAGE";

#[derive(Default)]
struct PositionHolder {
    orig: Option<usize>,
    decl: Option<usize>,
    spec: Option<usize>,
}

#[derive(Default)]
struct TransformTarget {
    export: PositionHolder,
    ident: PositionHolder,
    skip: bool,
}

struct PageTransformer {
    excluded: Vec<String>,

    props: TransformTarget,
    page: TransformTarget,

    has_init_props: bool,
    use_init_props: bool,
    keep_init_props: bool,

    has_multiple_props: bool,
}

pub fn transform_page(config: Config) -> impl VisitMut {
    PageTransformer {
        excluded: config.excluded,

        props: Default::default(),
        page: Default::default(),

        has_init_props: false,
        use_init_props: false,
        keep_init_props: false,

        has_multiple_props: false,
    }
}

impl VisitMut for PageTransformer {
    fn visit_mut_module_items(&mut self, items: &mut Vec<ModuleItem>) {
        self.find_page(items);

        if self.page.export.orig.is_none() {
            return;
        }

        self.find_ssg_prop(items);

        if self.props.export.orig.is_none() {
            if !self.use_init_props || self.has_multiple_props {
                return;
            }

            self.props.skip = true;
        }

        let mut new_items = vec![];

        let mut temp_page = None;

        for (pos, item) in items.iter_mut().enumerate() {
            if self.props.ident.orig.is_some()
                && pos == self.props.ident.orig.unwrap()
                && !self.props.skip
            {
                match item {
                    // gSSP = ..
                    // =>
                    // gSSP = wrap(.., excluded)
                    ModuleItem::Stmt(Stmt::Expr(ExprStmt { expr, .. })) => {
                        let assign_expr = expr.as_mut_assign().unwrap();

                        assign_expr.right =
                            assign_expr.right.take().wrap_props(self.excluded_expr());

                        new_items.push(item.take());
                    }
                    ModuleItem::Stmt(Stmt::Decl(decl)) => match decl {
                        // function gSSP ..
                        // =>
                        // const gSSP = wrap(.., excluded)
                        Decl::Fn(fn_decl) => {
                            *decl = fn_decl.take().as_wrapped_var_decl(self.excluded_expr());

                            new_items.push(item.take());
                        }
                        // const gSSP = ..
                        // =>
                        // const gSSP = wrap(.., excluded)
                        Decl::Var(var_decl) => {
                            let v = var_decl.decls.index_mut(self.props.ident.decl.unwrap());

                            v.init = Some(v.init.take().unwrap().wrap_props(self.excluded_expr()));

                            new_items.push(item.take());
                        }
                        _ => {}
                    },
                    // export function not_gSSP() ..
                    // =>
                    // export const not_gSSP = wrap(function not_gSSP()..)
                    ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
                        decl: export_decl,
                        ..
                    })) => match export_decl {
                        Decl::Fn(fn_decl) => {
                            *export_decl = fn_decl.take().as_wrapped_var_decl(self.excluded_expr());

                            new_items.push(item.take());
                        }
                        // export const not_gSSP = ..
                        // =>
                        // export const not_gSSP = wrap(..)
                        Decl::Var(var_decl) => {
                            let v = var_decl.decls.index_mut(self.props.ident.decl.unwrap());

                            v.init = Some(v.init.take().unwrap().wrap_props(self.excluded_expr()));

                            new_items.push(item.take());
                        }
                        _ => {}
                    },
                    // import { not_gSSP as gSSP } from '..' <-
                    // export { gSSP }
                    // =>
                    // import { not_gSSP as _NEXT_SUPERJSON_IMPORTED_PROPS } from '..'
                    // const  _NEXT_SUPERJSON_SSG_PROPS = wrap(_NEXT_SUPERJSON_IMPORTED_PROPS)
                    // export { _NEXT_SUPERJSON_SSG_PROPS as gSSP }
                    //
                    // import { not_gSSP } from '..' <-
                    // export { not_gSSP as gSSP }
                    // =>
                    // import { not_gSSP as _NEXT_SUPERJSON_IMPORTED_PROPS } from '..'
                    // const _NEXT_SUPERJSON_SSG_PROPS = wrap(_NEXT_SUPERJSON_IMPORTED_PROPS)
                    // export { _NEXT_SUPERJSON_SSG_PROPS as gSSP }
                    ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
                        specifiers, ..
                    })) => {
                        let s = specifiers
                            .index_mut(self.props.ident.spec.unwrap())
                            .as_mut_named()
                            .unwrap();

                        // imported: None, local: not_gSSP
                        // =>
                        // imported: not_gSSP, local: _NEXT_SUPERJSON_IMPORTED_PROPS
                        if s.imported.is_none() {
                            s.imported = Some(ModuleExportName::Ident(s.local.take()));
                        }

                        s.local = Ident::new_no_ctxt(NEXT_SSG_PROPS_LOCAL.into(), DUMMY_SP);

                        new_items.push(item.take());

                        new_items.push(temp_props_item(self.excluded_expr()));
                    }
                    _ => {}
                }
            } else {
                if !self.props.skip && pos == self.props.export.orig.unwrap() {
                    match item {
                        ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
                            decl: export_decl,
                            ..
                        })) => {
                            match export_decl {
                                // export function gSSP..
                                // =>
                                // export const gSSP = wrap(.., excluded)
                                Decl::Fn(fn_decl) => {
                                    *export_decl =
                                        fn_decl.take().as_wrapped_var_decl(self.excluded_expr());
                                }
                                // export const gSSP = ..
                                // =>
                                // export const gSSP = wrap(.., excluded)
                                Decl::Var(var_decl) => {
                                    let v =
                                        var_decl.decls.index_mut(self.props.export.decl.unwrap());

                                    v.init = Some(
                                        v.init.take().unwrap().wrap_props(self.excluded_expr()),
                                    );
                                }
                                _ => {}
                            }
                        }

                        ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(NamedExport {
                            specifiers,
                            src,
                            ..
                        })) => {
                            // export { not_gSSP as gSSP } from '..'
                            // =>
                            // import { not_gSSP as _NEXT_SUPERJSON_IMPORTED_PROPS } from '..'
                            // const _NEXT_SUPERJSON_SSG_PROPS = wrap(_NEXT_SUPERJSON_IMPORTED_PROPS, excluded)
                            // export { _NEXT_SUPERJSON_SSG_PROPS as gSSP }
                            if let Some(src) = src {
                                let s = specifiers
                                    .index_mut(self.props.export.spec.unwrap())
                                    .as_mut_named()
                                    .take()
                                    .unwrap();

                                new_items.push(temp_import_item(
                                    s.orig.clone(),
                                    NEXT_SSG_PROPS_LOCAL,
                                    src,
                                ));

                                new_items.push(temp_props_item(self.excluded_expr()));

                                new_items.push(ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(
                                    NamedExport {
                                        with: None,
                                        span: DUMMY_SP,
                                        specifiers: vec![ExportSpecifier::Named(
                                            ExportNamedSpecifier {
                                                exported: s.exported.take(),
                                                is_type_only: false,
                                                orig: ModuleExportName::Ident(Ident::new_no_ctxt(
                                                    NEXT_SSG_PROPS_ORIG.into(),
                                                    DUMMY_SP,
                                                )),
                                                span: DUMMY_SP,
                                            },
                                        )],
                                        src: None,
                                        type_only: false,
                                    },
                                )));

                                specifiers.remove(self.props.export.spec.unwrap());

                            // export { gSSP }
                            // export { not_gSSP as gSSP }
                            // =>
                            // export { _NEXT_SUPERJSON_SSG_PROPS as gSSP }
                            } else {
                                let s = specifiers
                                    .index_mut(self.props.export.spec.unwrap())
                                    .as_mut_named()
                                    .unwrap();

                                if s.exported.is_none() {
                                    s.exported = Some(s.orig.clone());
                                }

                                // case 1: imported
                                // import { not_gSSP as _NEXT_SUPERJSON_IMPORTED_PROPS }
                                // => _NEXT_SUPERJSON_SSG_PROPS
                                //
                                // case 2: local
                                // const gSSP = () => {}
                                // => gSSP
                                if self.props.ident.spec.is_some() {
                                    s.orig = ModuleExportName::Ident(Ident::new_no_ctxt(
                                        NEXT_SSG_PROPS_ORIG.into(),
                                        DUMMY_SP,
                                    ));
                                }
                            }
                        }
                        _ => {}
                    }
                }

                let mut keep_page = false;

                if pos == self.page.export.orig.unwrap() && !self.page.skip {
                    match item {
                        ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultExpr(
                            ExportDefaultExpr { expr, .. },
                        )) => {
                            keep_page = true;
                            *expr = expr.take().wrap_page();
                        }
                        ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultDecl(
                            ExportDefaultDecl { decl, .. },
                        )) => {
                            keep_page = true;
                            // TODO: remove duplicate code
                            match decl {
                                DefaultDecl::Class(class_expr) => {
                                    if class_expr.ident.is_some() {
                                        let id = class_expr.ident.as_ref().unwrap().clone();

                                        new_items.push(ModuleItem::Stmt(Stmt::Decl(
                                            class_expr.take().as_class_decl().unwrap().into(),
                                        )));

                                        *item = ModuleItem::ModuleDecl(
                                            ModuleDecl::ExportDefaultExpr(ExportDefaultExpr {
                                                expr: Box::new(Expr::Ident(id)).wrap_page(),
                                                span: DUMMY_SP,
                                            }),
                                        );
                                    } else {
                                        let expr: Box<Expr> = Box::new(class_expr.take().into());

                                        *item = ModuleItem::ModuleDecl(
                                            ModuleDecl::ExportDefaultExpr(ExportDefaultExpr {
                                                expr: expr.wrap_page(),
                                                span: DUMMY_SP,
                                            }),
                                        );
                                    }
                                }
                                DefaultDecl::Fn(fn_expr) => {
                                    if fn_expr.ident.is_some() {
                                        let id = fn_expr.ident.as_ref().unwrap().clone();

                                        new_items.push(ModuleItem::Stmt(Stmt::Decl(
                                            fn_expr.take().as_fn_decl().unwrap().into(),
                                        )));

                                        *item = ModuleItem::ModuleDecl(
                                            ModuleDecl::ExportDefaultExpr(ExportDefaultExpr {
                                                expr: Box::new(Expr::Ident(id)).wrap_page(),
                                                span: DUMMY_SP,
                                            }),
                                        );
                                    } else {
                                        let expr: Box<Expr> = Box::new(fn_expr.take().into());

                                        *item = ModuleItem::ModuleDecl(
                                            ModuleDecl::ExportDefaultExpr(ExportDefaultExpr {
                                                expr: expr.wrap_page(),
                                                span: DUMMY_SP,
                                            }),
                                        );
                                    }
                                }
                                _ => {}
                            }
                        }
                        ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(NamedExport {
                            specifiers,
                            src,
                            ..
                        })) => {
                            let s = specifiers
                                .index_mut(self.page.export.spec.unwrap())
                                .as_mut_named()
                                .take()
                                .unwrap();

                            // export { unwrapped as default } from 'src'
                            // =>
                            // import { unwrapped as _NEXT_SUPERJSON_IMPORTED_PAGE } from 'src'
                            // export default wrap(_NEXT_SUPERJSON_IMPORTED_PAGE)
                            if let Some(src) = src {
                                new_items.push(temp_import_item(
                                    s.orig.clone(),
                                    NEXT_PAGE_LOCAL,
                                    src,
                                ));

                                let new_page = ModuleItem::ModuleDecl(
                                    ModuleDecl::ExportDefaultExpr(ExportDefaultExpr {
                                        expr: Box::new(Expr::Ident(Ident::new_no_ctxt(
                                            NEXT_PAGE_LOCAL.into(),
                                            DUMMY_SP,
                                        )))
                                        .wrap_page(),
                                        span: DUMMY_SP,
                                    }),
                                );
                                temp_page = Some(new_page);

                            // export { Page as default }
                            // =>
                            // export default wrap(Page, excluded)
                            } else {
                                if let ModuleExportName::Ident(id) = &s.orig {
                                    let new_page = ModuleItem::ModuleDecl(
                                        ModuleDecl::ExportDefaultExpr(ExportDefaultExpr {
                                            expr: Box::new(Expr::Ident(id.clone())).wrap_page(),
                                            span: DUMMY_SP,
                                        }),
                                    );
                                    temp_page = Some(new_page);
                                }
                            }

                            specifiers.remove(self.page.export.spec.unwrap());
                        }
                        _ => {}
                    }
                }

                match item {
                    ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(NamedExport {
                        specifiers,
                        ..
                    })) => {
                        if !specifiers.is_empty() {
                            if !keep_page {
                                new_items.push(item.take());
                            } else {
                                temp_page = Some(item.take());
                            }
                        }
                    }
                    _ => {
                        if !keep_page {
                            new_items.push(item.take());
                        } else {
                            temp_page = Some(item.take());
                        }
                    }
                }
            }
        }

        if let Some(tmp) = temp_page {
            new_items.push(tmp);
        }

        // TODO: these two stmts can be combined
        if !self.props.skip {
            prepend_stmt(
                &mut new_items,
                superjson_import_decl(SUPERJSON_PROPS_IMPORTED),
            );
        }
        if self.use_init_props {
            prepend_stmt(
                &mut new_items,
                superjson_import_decl(SUPERJSON_INIT_PROPS_IMPORTED),
            );
        }
        if !self.page.skip {
            prepend_stmt(
                &mut new_items,
                superjson_import_decl(SUPERJSON_PAGE_IMPORTED),
            );
        }

        *items = new_items;
    }

    fn visit_mut_class_member(&mut self, member: &mut ClassMember) {
        member.visit_mut_children_with(self);
        match member {
            ClassMember::ClassProp(p) => {
                if let PropName::Ident(id) = &p.key {
                    if &*id.sym == INITIAL_PROPS {
                        if let Some(expr) = &mut p.value {
                            self.use_init_props = true;
                            if !self.keep_init_props {
                                p.value = Some(expr.take().wrap_init_props(self.excluded_expr()));
                            }
                        }
                    }
                }
            }
            ClassMember::Method(m) => {
                if let PropName::Ident(id) = &m.key {
                    if &*id.sym == INITIAL_PROPS {
                        self.use_init_props = true;
                        if !self.keep_init_props {
                            *member = ClassMember::ClassProp(ClassProp {
                                accessibility: m.accessibility.take(),
                                declare: false,
                                decorators: vec![],
                                definite: false,
                                is_abstract: m.is_abstract,
                                is_optional: m.is_optional,
                                is_override: m.is_override,
                                is_static: m.is_static,
                                key: m.key.take(),
                                readonly: false,
                                span: DUMMY_SP,
                                type_ann: None,
                                value: Some(
                                    Box::new(Expr::Fn(FnExpr {
                                        function: m.function.take(),
                                        ident: None,
                                    }))
                                    .wrap_init_props(self.excluded_expr()),
                                ),
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn visit_mut_assign_expr(&mut self, a: &mut AssignExpr) {
        a.visit_mut_children_with(self);

        if a.left.is_simple() {
            if let Some(mut expr) = a.left.take().simple() {
                if let Some(MemberExpr { prop, .. }) = expr.as_mut_member() {
                    prop.visit_mut_children_with(self);
                }

                if self.has_init_props {
                    if !self.keep_init_props {
                        a.right = a.right.take().wrap_init_props(self.excluded_expr());
                    }
                    self.use_init_props = true;
                    self.has_init_props = false;
                }

                a.left = AssignTarget::Simple(expr);
            }
        }
    }

    fn visit_mut_member_prop(&mut self, p: &mut MemberProp) {
        p.visit_mut_children_with(self);

        if let Some(id) = p.as_ident() {
            if &*id.sym == INITIAL_PROPS {
                self.has_init_props = true;
            }
        }
    }
}

impl PageTransformer {
    pub fn excluded_expr(&mut self) -> ExprOrSpread {
        ExprOrSpread {
            spread: None,
            expr: Box::new(Expr::Array(ArrayLit {
                span: DUMMY_SP,
                elems: self
                    .excluded
                    .iter()
                    .map(|e| {
                        Some(ExprOrSpread {
                            spread: None,
                            expr: Box::new(Expr::Lit(Lit::Str(Str {
                                span: DUMMY_SP,
                                value: e.clone().into(),
                                raw: None,
                            }))),
                        })
                    })
                    .collect(),
            })),
        }
    }

    pub fn find_ssg_prop(&mut self, items: &mut Vec<ModuleItem>) {
        let mut ssg_prop_ident = None;

        let mut first = None;

        items.iter_mut().enumerate().any(|(pos, item)| {
            let found = match item {
                // check has ssg props
                ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl { decl, .. })) => {
                    match decl {
                        Decl::Fn(fn_decl) => SSG_EXPORTS.contains(&&*fn_decl.ident.sym),
                        Decl::Var(var_decl) => {
                            let pos = var_decl.decls.iter().position(|decl| {
                                SSG_EXPORTS.contains(&&*decl.name.as_ident().unwrap().sym)
                            });

                            if self.props.export.decl.is_none() {
                                self.props.export.decl = pos;
                            }

                            pos.is_some()
                        }
                        _ => false,
                    }
                }
                ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(NamedExport {
                    specifiers,
                    src,
                    ..
                })) => {
                    let pos = specifiers.iter().position(|specifier| match specifier {
                        ExportSpecifier::Named(ExportNamedSpecifier {
                            orig: ModuleExportName::Ident(orig_id),
                            exported,
                            ..
                        }) => {
                            let exported_as = match exported {
                                Some(ModuleExportName::Ident(exported_id)) => &exported_id.sym,
                                _ => &orig_id.sym,
                            };

                            if SSG_EXPORTS.contains(&&**exported_as) {
                                self.props.skip = src.is_some()
                                    && (exported.is_none() || (&&**exported_as == &&*orig_id.sym));

                                if !self.props.skip {
                                    ssg_prop_ident = Some((*orig_id.sym).to_string());
                                }
                                return true;
                            }
                            false
                        }
                        _ => false,
                    });

                    if self.props.export.spec.is_none() {
                        self.props.export.spec = pos;
                    }

                    pos.is_some()
                }
                _ => false,
            };

            if found {
                if first.is_some() {
                    self.has_multiple_props = true;
                    return true;
                }
                first = Some(pos);
            }

            false
        });

        if self.has_multiple_props {
            return;
        }

        self.keep_init_props = first.is_some();

        // check initial props
        items
            .iter_mut()
            .for_each(|item| item.visit_mut_children_with(self));

        if first.is_some() && self.use_init_props {
            self.has_multiple_props = true;
            return;
        }

        self.props.export.orig = first;

        if ssg_prop_ident.is_some() && !self.props.skip {
            let mut n = items.len();

            while n > 0 {
                n -= 1;

                if self.props.export.orig.unwrap() == n {
                    continue;
                }

                match &items[n] {
                    // gSSP = ..
                    ModuleItem::Stmt(Stmt::Expr(ExprStmt { expr, .. })) => {
                        if expr.is_assign() {
                            let assign = expr.as_assign().unwrap();

                            let left = assign.left.as_ident();

                            if left.is_some() {
                                if assign.op == op!("=")
                                    && &*left.unwrap().sym == ssg_prop_ident.as_ref().unwrap()
                                {
                                    self.props.ident.orig = Some(n);
                                    break;
                                }
                            }
                        }
                    }
                    // function gSSP() ..
                    // const gSSP = ..
                    ModuleItem::Stmt(Stmt::Decl(decl)) => match decl {
                        Decl::Fn(fn_decl) => {
                            if &*fn_decl.ident.sym == ssg_prop_ident.as_ref().unwrap() {
                                self.props.ident.orig = Some(n);
                                break;
                            }
                        }
                        Decl::Var(var_decl) => {
                            self.props.ident.decl = var_decl.decls.iter().position(|decl| {
                                let id = decl.name.as_ident();

                                if id.is_some()
                                    && &*id.unwrap().sym == ssg_prop_ident.as_ref().unwrap()
                                {
                                    self.props.ident.orig = Some(n);
                                    return true;
                                }

                                false
                            });

                            if self.props.ident.decl.is_some() {
                                break;
                            }
                        }
                        _ => {}
                    },

                    // export function not_gSSP() ..
                    // export const not_gSSP = ..
                    ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
                        decl: export_decl,
                        ..
                    })) => match export_decl {
                        Decl::Fn(fn_decl) => {
                            if &*fn_decl.ident.sym == ssg_prop_ident.as_ref().unwrap() {
                                self.props.ident.orig = Some(n);
                                break;
                            }
                        }
                        Decl::Var(var_decl) => {
                            self.props.ident.decl = var_decl.decls.iter().position(|decl| {
                                let id = decl.name.as_ident();

                                if id.is_some()
                                    && &*id.unwrap().sym == ssg_prop_ident.as_ref().unwrap()
                                {
                                    self.props.ident.orig = Some(n);
                                    return true;
                                }

                                false
                            });

                            if self.props.ident.decl.is_some() {
                                break;
                            }
                        }
                        _ => {}
                    },
                    // import { gSSP } from '..'
                    ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
                        specifiers, ..
                    })) => {
                        self.props.ident.spec = specifiers.iter().position(|specifier| {
                            if let ImportSpecifier::Named(ImportNamedSpecifier {
                                local,
                                imported,
                                ..
                            }) = specifier
                            {
                                if &*local.sym == ssg_prop_ident.as_ref().unwrap() {
                                    if imported.is_some() {
                                        if let ModuleExportName::Ident(ident) =
                                            imported.as_ref().unwrap()
                                        {
                                            self.props.skip = SSG_EXPORTS.contains(&&*ident.sym);
                                        }
                                    }

                                    self.props.ident.orig = Some(n);
                                    return true;
                                }
                            }
                            false
                        });

                        if self.props.ident.orig.is_some() {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn find_page(&mut self, items: &Vec<ModuleItem>) {
        self.page.export.orig = items.iter().position(|item| match item {
            // check has page
            ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultExpr(_)) => true,
            ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultDecl(_)) => true,
            ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(NamedExport {
                specifiers,
                src,
                ..
            })) => {
                self.page.export.spec = specifiers.iter().position(|spec| match spec {
                    ExportSpecifier::Named(ExportNamedSpecifier {
                        orig: ModuleExportName::Ident(Ident { sym, .. }),
                        exported,
                        ..
                    }) => match exported {
                        Some(ModuleExportName::Ident(Ident {
                            sym: exported_sym, ..
                        })) => {
                            self.page.skip =
                                exported_sym == "default" && sym == "default" && src.is_some();
                            exported_sym == "default"
                        }
                        _ => {
                            // export { default } from 'source' -> skip
                            self.page.skip = src.is_some() && sym == "default";
                            self.page.skip
                        }
                    },
                    _ => false,
                });

                self.page.export.spec.is_some()
            }
            _ => false,
        })
    }
}
