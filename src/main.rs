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

fn main() {
    print_example_html_using_maud()
}
