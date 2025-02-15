use swc_core::{
    common::{util::take::Take, DUMMY_SP},
    ecma::{ast::*, utils::ExprFactory},
};

use crate::page::{
    NEXT_SSG_PROPS_LOCAL, NEXT_SSG_PROPS_ORIG, SUPERJSON_INIT_PROPS_LOCAL, SUPERJSON_PAGE_LOCAL,
    SUPERJSON_PROPS_LOCAL,
};

pub fn superjson_import_decl(superjson_import_name: &str) -> ModuleItem {
    ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
        with: None,
        phase: ImportPhase::Evaluation,
        span: DUMMY_SP,
        type_only: false,
        specifiers: vec![ImportSpecifier::Named(ImportNamedSpecifier {
            local: Ident::new_no_ctxt(format!("_{superjson_import_name}").into(), DUMMY_SP),
            span: DUMMY_SP,
            imported: Some(ModuleExportName::Ident(Ident::new_no_ctxt(
                superjson_import_name.into(),
                DUMMY_SP,
            ))),
            is_type_only: false,
        })],
        src: Box::new(Str {
            span: DUMMY_SP,
            value: "next-superjson-plugin/tools".into(),
            raw: None,
        }),
    }))
}

pub fn temp_props_item(excluded: ExprOrSpread) -> ModuleItem {
    ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
        declare: false,
        decls: vec![VarDeclarator {
            definite: false,
            init: Some(
                Box::new(Expr::Ident(Ident::new_no_ctxt(
                    NEXT_SSG_PROPS_LOCAL.into(),
                    DUMMY_SP,
                )))
                .wrap_props(excluded),
            ),
            name: Pat::Ident(BindingIdent {
                id: Ident::new_no_ctxt(NEXT_SSG_PROPS_ORIG.into(), DUMMY_SP),
                type_ann: None,
            }),
            span: DUMMY_SP,
        }],
        kind: VarDeclKind::Const,
        span: DUMMY_SP,
        ..Default::default()
    }))))
}

pub fn temp_import_item(imported: ModuleExportName, local: &str, src: &mut Str) -> ModuleItem {
    ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
        with: None,
        phase: ImportPhase::Evaluation,
        span: DUMMY_SP,
        specifiers: vec![ImportSpecifier::Named(ImportNamedSpecifier {
            imported: Some(imported),
            is_type_only: false,
            local: Ident::new_no_ctxt(local.into(), DUMMY_SP),
            span: DUMMY_SP,
        })],
        // should clone
        src: Box::new(src.clone()),
        type_only: false,
    }))
}

pub trait Wrapper {
    fn wrap_props(self, excluded: ExprOrSpread) -> Box<Expr>;
    fn wrap_init_props(self, excluded: ExprOrSpread) -> Box<Expr>;
    fn wrap_page(self) -> Box<Expr>;
}

impl Wrapper for Box<Expr> {
    fn wrap_props(self, excluded: ExprOrSpread) -> Box<Expr> {
        Box::new(Expr::Call(CallExpr {
            args: vec![self.as_arg(), excluded],
            callee: Ident::new_no_ctxt(SUPERJSON_PROPS_LOCAL.into(), DUMMY_SP).as_callee(),
            span: DUMMY_SP,
            type_args: None,
            ..Default::default()
        }))
    }
    fn wrap_init_props(self, excluded: ExprOrSpread) -> Box<Expr> {
        Box::new(Expr::Call(CallExpr {
            args: vec![self.as_arg(), excluded],
            callee: Ident::new_no_ctxt(SUPERJSON_INIT_PROPS_LOCAL.into(), DUMMY_SP).as_callee(),
            span: DUMMY_SP,
            type_args: None,
            ..Default::default()
        }))
    }
    fn wrap_page(self) -> Box<Expr> {
        Box::new(Expr::Call(CallExpr {
            args: vec![self.as_arg()],
            callee: Ident::new_no_ctxt(SUPERJSON_PAGE_LOCAL.into(), DUMMY_SP).as_callee(),
            span: DUMMY_SP,
            type_args: None,
            ..Default::default()
        }))
    }
}

pub trait DeclUtil {
    fn as_wrapped_var_decl(self, excluded: ExprOrSpread) -> Decl;
}

impl DeclUtil for FnDecl {
    fn as_wrapped_var_decl(mut self, excluded: ExprOrSpread) -> Decl {
        Decl::Var(Box::new(VarDecl {
            declare: false,
            decls: vec![VarDeclarator {
                definite: false,
                init: Some(
                    Box::new(Expr::Fn(FnExpr {
                        function: self.function.take(),
                        ident: None,
                    }))
                    .wrap_props(excluded),
                ),
                name: Pat::Ident(BindingIdent {
                    id: self.ident.take(),
                    type_ann: None,
                }),
                span: DUMMY_SP,
            }],
            kind: VarDeclKind::Const,
            span: DUMMY_SP,
            ..Default::default()
        }))
    }
}
