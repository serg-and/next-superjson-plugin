use std::path::PathBuf;
use swc_core::ecma::{
    parser::{EsSyntax, Syntax},
    transforms::testing::{test_fixture, FixtureTestConfig},
    visit::visit_mut_pass,
};
use testing::fixture;

use next_superjson::{app::transform_app, page::transform_page, Config, Router};

#[fixture("tests/fixture/page/**/code.js")]
fn fixture_page(input: PathBuf) {
    let output = input.with_file_name("output.js");

    test_fixture(
        Syntax::Es(EsSyntax {
            jsx: true,
            ..Default::default()
        }),
        &|_| {
            visit_mut_pass(transform_page(Config {
                router: Router::Page,
                excluded: vec!["smth".to_string()],
            }))
        },
        &input,
        &output,
        FixtureTestConfig::default(),
    );
}

#[fixture("tests/fixture/app/**/code.js")]
fn fixture_app(input: PathBuf) {
    let output = input.with_file_name("output.js");

    test_fixture(
        Syntax::Es(EsSyntax {
            jsx: true,
            ..Default::default()
        }),
        &|_| {
            visit_mut_pass(transform_app(Config {
                router: Router::App,
                excluded: vec!["smth".to_string()],
            }))
        },
        &input,
        &output,
        FixtureTestConfig::default(),
    );
}
