use serde::Deserialize;
use swc_core::{
    ecma::{ast::*, visit::*},
    plugin::{plugin_transform, proxies::TransformPluginProgramMetadata},
};

use app::*;
use page::*;

pub mod app;
pub mod page;
mod utils;

static MISSING_CONFIG: &str = "next-superjson-plugin: Missing config,
Provide a plugin config in your next.config.(js|ts) like:

experimental: {
  swcPlugins: [
    ['next-superjson-plugin', { router: 'APP' | 'PAGE' }]
  ],
},
";

static BAD_CONFIG: &str = "next-superjson-plugin: Failed to parse config,
Provide a plugin config in your next.config.(js|ts) like:

experimental: {
  swcPlugins: [
    ['next-superjson-plugin', { router: 'APP' | 'PAGE' }]
  ],
},
";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Router {
    #[serde(alias = "app", alias = "App")]
    App,
    #[serde(alias = "page", alias = "Page")]
    Page,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    pub router: Router,
    // disabled because of Next.js bug, see below
    // #[serde(default)]
    // pub excluded: Vec<String>,
}

// pub enum DirType {
//     Page,
//     App,
// }

#[plugin_transform]
pub fn process_transform(program: Program, metadata: TransformPluginProgramMetadata) -> Program {
    let plugin_config_str = &metadata
        .get_transform_plugin_config()
        .expect(MISSING_CONFIG);
    let config = serde_json::from_str::<Config>(&plugin_config_str).expect(BAD_CONFIG);

    match config.router {
        Router::App => program.apply(&mut visit_mut_pass(transform_app(config))),
        Router::Page => program.apply(&mut visit_mut_pass(transform_page(config))),
    }

    /*
    // Automatic detection of router type is disabled because of a bug in Next.js
    // Re enable this code once Next.js fixes this issue
    // https://github.com/vercel/next.js/issues/72019

    let raw_cwd = _metadata
        .get_context(&TransformPluginMetadataContextKind::Cwd)
        .unwrap();

    let raw_path = _metadata
        .get_context(&TransformPluginMetadataContextKind::Filename)
        .unwrap();

    // Windows path separator -> Unix path separator
    let cwd = &raw_cwd.replace('\\', "/");
    let path = &raw_path.replace('\\', "/");

    // overlapping prefix
    let prefix = cwd
        .chars()
        .zip(path.chars())
        .take_while(|(a, b)| a == b)
        .map(|(a, _)| a)
        .collect::<String>();

    if let Some(relative_path) = path.strip_prefix(&prefix) {
        let mut is_page = false;

        for component in Path::new(relative_path).components() {
            match component {
                Component::Normal(str) => match str.to_str().unwrap_or_default() {
                    // skip non-source stuff
                    "node_modules" => {
                        return program;
                    }
                    "pages" => {
                        is_page = true;
                        break;
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        // consider server components outside the app directory
        let dir_type = if is_page { DirType::Page } else { DirType::App };

        let config = serde_json::from_str::<Config>(
            &_metadata
                .get_transform_plugin_config()
                .unwrap_or_else(|| "{}".to_string()),
        )
        .expect("Failed to parse plugin config");

        match dir_type {
            DirType::Page => program.fold_with(&mut as_folder(transform_page(config))),
            DirType::App => program.fold_with(&mut as_folder(transform_app(config))),
        }
    } else {
        program
    }
    */
}
