use maud::html;

fn main() {
    let content = html! {
        h1 { "Hello, world!" }
        p.intro {
            "This is an example of the "
            a href="https://github.com/lambda-fairy/maud" { "Maud" }
            " template language."
        }
    };

    println!("{}", content.into_string());
}
