use maud::html;
use maud::PreEscaped;

pub mod reusingthewheel_theme {
    use std::collections::HashMap;
    use chrono::{Date, DateTime, Utc};
    use maud::{html, Markup, DOCTYPE, PreEscaped};
    use crate::{Link, Page, PageConfig, PageType, Website};

    fn header(website: &Website) -> Markup {
        return html! {
            header {
                a href=(website.base_url) { (website.title) }
                nav {
                    ul {
                        @for item in &website.menu_items {
                            li {
                                a href=(item.url) { (item.title) }
                            }
                        }
                    }
                }
            }
        }
    }

    fn footer(website: &Website) -> Markup {
        return html! {
            footer {
                p {
                    (PreEscaped("&copy"))(website.year)(PreEscaped("&nbsp;"))a href=(website.base_url) { (website.title) }
                }
            }
        }
    }

    fn baseof(website: &Website, main_content: Markup) -> Markup {
        return html! {
             (DOCTYPE)
            html lang="en-us" {
                head {
                    meta charset="utf-8";
                    meta name="viewport" content="width=device-width, initial-scale=1";
                    meta http-equiv="X-UA-Compatible" content="IE=edge";
                    meta name="description" content=(website.description);
                    meta name="author" content=(website.author);
                    title {(website.title)}
                    link rel="stylesheet" href="css/style.css";
                }
                body {
                    (header(website))
                    main {
                        (main_content)
                    }
                    (footer(website))
                }
            }
        }
    }

    pub fn single_page(website: &Website, page: &Page, page_content: String) -> Markup {
        let pre_escaped_page_content = PreEscaped(page_content);
        let content = html! {
            article {
                h1 {
                    (page.config.title)
                }
                @if let Some(date) = page.config.date {
                    time {
                        (date.format("%Y.%m.%d"))
                    }
                } @else {
                }
                div {
                    (pre_escaped_page_content)
                }
                div {
                    ul id="tags" {
                        @for link in &website.get_category_links_for_page(page) {
                            li {
                                a href=(link.url) { (link.title) }
                            }
                        }
                    }
                }
            }
        };
        return baseof(website, content);
    }

    pub fn archive(website: &Website, title: &str, year_to_articles: Vec<(String, Vec<&Page>)>) -> Markup {
        let content = html! {
            h1 {
                (title)
            }
            div {
                ul id="articles" {
                    @for (year, articles) in year_to_articles.into_iter() {
                        li {
                            h2 {
                                (year)
                            }
                            @for article in articles {
                                li {
                                    time {
                                        (article.config.date.unwrap().format("%Y.%m.%d"))
                                    } (PreEscaped("&nbsp;"))
                                      a href=(article.config.path) { (article.config.title) }
                                }
                            }
                        }
                    }
                }
            }
        };

        return baseof(website, content)
    }

    fn taxonomy_item_list(title: &str, links: Vec<Link>) -> Markup {
        return html! {
            article {
                h1 {
                    (title)
                }
                div {
                    ul id="tags" {
                        @for link in links {
                            li {
                                a href=(link.url) { (link.title) }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn category_list(website: &Website, title: &str, links: Vec<Link>) -> Markup {
        let content = taxonomy_item_list(title, links);
        return baseof(website, content);
    }

    pub fn project_list(website: &Website, title: &str, links: Vec<Link>) -> Markup {
        let content = taxonomy_item_list(title, links);
        return baseof(website, content);
    }
}