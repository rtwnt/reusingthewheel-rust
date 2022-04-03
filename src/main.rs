use std::fs;
use comrak::{Arena, ComrakExtensionOptions, ComrakOptions, ComrakParseOptions, ComrakRenderOptions, format_html, parse_document};
use comrak::nodes::{AstNode, NodeValue};
use maud::html;

fn print_example_html_using_maud() {
    let example_html = html! {
        h1 { "Hello, world!" }
        p.intro {
            "This is an example of the "
            a href="https://github.com/lambda-fairy/maud" { "Maud" }
            " template language."
        }
    };

    println!("{}", example_html.into_string());
}

pub fn print_example_html_using_comrak(filename: &str) {
    let contents = fs::read_to_string(filename)
        .expect("Something went wrong reading the file");

    // The returned nodes are created in the supplied Arena, and are bound by its lifetime.
    let arena = Arena::new();

    let options = ComrakOptions {
        extension: ComrakExtensionOptions {
            strikethrough: false,
            tagfilter: false,
            table: true,
            autolink: false,
            tasklist: false,
            superscript: false,
            header_ids: Some("abc".to_string()),
            footnotes: true,
            description_lists: false,
            front_matter_delimiter: Some("---".to_string()),
        },
        parse: ComrakParseOptions {
            smart: false,
            default_info_string: Some("abc".to_string()),
        },
        render: ComrakRenderOptions {
            hardbreaks: false,
            github_pre_lang: false,
            width: 123456,
            unsafe_: false,
            escape: false,
        },
    };

    let root = parse_document(
        &arena,
        &contents,
        &options);

    fn iter_nodes<'a, F>(node: &'a AstNode<'a>, f: &F)
        where F : Fn(&'a AstNode<'a>) {
        f(node);
        for c in node.children() {
            iter_nodes(c, f);
        }
    }

    iter_nodes(root, &|node| {
        match &mut node.data.borrow_mut().value {
            &mut NodeValue::Text(ref mut text) => {
                let orig = std::mem::replace(text, vec![]);
                *text = String::from_utf8(orig).unwrap().replace("my", "your").as_bytes().to_vec();
            },
            &mut NodeValue::FrontMatter(ref mut text) => {
                println!("Front matter:\n {}", String::from_utf8_lossy(text));
            },
            _ => (),
        }
    });

    let mut html = vec![];
    format_html(root, &ComrakOptions::default(), &mut html).unwrap();

    println!("{}", String::from_utf8(html).unwrap());
}

fn main() {
    print_example_html_using_maud();
    // Reference links work in comrak, but including Hugo-style references breaks them
    print_example_html_using_comrak("src/content/posts/a-new-blog-engine-project.md");
}
