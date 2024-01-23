use clap::Parser;
use dateparser::DateTimeUtc;
use handlebars::RenderError;
use handlebars::{handlebars_helper, Handlebars, JsonRender};
use markdown::mdast::Node;
use markdown::mdast::Node::{
    BlockQuote, Break, Code, Delete, Emphasis, FootnoteDefinition, FootnoteReference, Heading,
    Html, Image, ImageReference, InlineCode, InlineMath, Link, LinkReference, List, ListItem, Math,
    MdxFlowExpression, MdxJsxFlowElement, MdxJsxTextElement, MdxTextExpression, MdxjsEsm,
    Paragraph, Root, Strong, Text, ThematicBreak, Toml, Yaml,
};
use serde_json::json;
use std::cmp::Ordering;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::ErrorKind;
use std::path::Path;
use std::path::PathBuf;
use time::format_description::parse;
use time::OffsetDateTime;
use toml::Value;
extern crate fs_extra;
use chrono::Local;
use chrono::Utc;
use chrono::{DateTime, Datelike};
use fs_extra::dir::CopyOptions;
use fs_extra::TransitProcess;
use handlebars::JsonValue;
use handlebars::ScopedJson;
use markdown::{Constructs, Options, ParseOptions};
use rust_embed::{EmbeddedFile, RustEmbed};
use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};
use words_count::WordsCount;
#[derive(RustEmbed)]
#[folder = "template_src/"]
struct Template;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Output folder
    #[arg(short, long, default_value_t = String::from("dist"))]
    target: String,

    /// Manifest file
    #[arg(short, long, default_value_t = String::from("quipquick.toml"))]
    manifest: String,

    #[arg(short, long)]
    prefix: Option<String>,
}

#[derive(Debug, Clone)]
struct Post {
    date: DateTime<Utc>,
    description: String,
    md: String,
    title: String,
    tags: Vec<String>,
    word_count: usize,
    repo: String,
    blog_title: String,
    quipquick_version: String,
    current_time: String,
}

impl Serialize for Post {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(6)).unwrap();

        map.serialize_entry(
            "date",
            format!(
                "{}-{:0width$}-{}",
                self.date.year(),
                self.date.month(),
                self.date.day(),
                width = 2
            )
            .as_str(),
        )
        .unwrap();
        map.serialize_entry("description", &self.description)
            .unwrap();
        map.serialize_entry("md", &self.md).unwrap();
        map.serialize_entry("title", &self.title).unwrap();
        map.serialize_entry("tags", &self.tags).unwrap();
        map.serialize_entry("word_count", &self.word_count).unwrap();
        map.serialize_entry("repo", &self.repo).unwrap();
        map.serialize_entry("blog_title", &self.blog_title).unwrap();
        map.serialize_entry("quipquick_version", &self.quipquick_version)
            .unwrap();
        map.serialize_entry("current_time", &self.current_time)
            .unwrap();
        map.end()
    }
}

fn render_markdown(
    node: &Node,
    output: &mut String,
    img_path: &mut Vec<String>,
    count: &mut usize,
) {
    match node {
        Paragraph(p) => {
            output.push_str("<p>");

            for n in node.children().unwrap() {
                render_markdown(n, output, img_path, count);
            }

            output.push_str("</p>");
        }
        Text(t) => {
            *count += words_count::count(&t.value).words;
            output.push_str(&t.value);
        }
        Root(r) => {
            for n in &r.children {
                render_markdown(n, output, img_path, count);
            }
        }
        BlockQuote(b) => {
            output.push_str("<blockquote>");

            for n in &b.children {
                render_markdown(n, output, img_path, count);
            }

            output.push_str("</blockquote>");
        }

        FootnoteDefinition(_) => {}
        MdxJsxFlowElement(_) => {}
        MdxjsEsm(_) => {}
        List(l) => {
            if l.ordered {
                output.push_str("<ol>");
            } else {
                output.push_str("<ul>");
            }

            for n in &l.children {
                render_markdown(n, output, img_path, count);
            }

            if l.ordered {
                output.push_str("</ol>");
            } else {
                output.push_str("</ul>");
            }
        }
        Toml(c) => {
            println!("{:?}", c);
        }
        Yaml(c) => {
            println!("{:?}", c);
        }
        Break(_) => {
            output.push_str("<br />");
        }
        InlineCode(ic) => {
            output.push_str("<code>");
            output.push_str(&ic.value);
            output.push_str("</code>");
        }
        InlineMath(im) => {
            output.push_str("<code class=\"language-math math-inline\">");
            output.push_str(&im.value);
            output.push_str("</code>");
        }
        Delete(_) => {}
        Emphasis(e) => {
            output.push_str("<em>");
            for n in &e.children {
                render_markdown(n, output, img_path, count);
            }
            output.push_str("</em>");
        }
        MdxTextExpression(_) => {}
        FootnoteReference(_) => {}
        Html(h) => {
            output.push_str(&h.value);
        }
        Image(i) => {
            output.push_str("<div class=\"img-container\">");
            output.push_str(
                format!(
                    "<img class=\"img\" src=\"{}\" alt=\"{}\" />",
                    &i.url, &i.alt
                )
                .as_str(),
            );
            if i.alt.len() > 0 {
                output.push_str("<div class=\"img-title\">");
                output.push_str(&i.alt);
                output.push_str("</div>");
            }
            output.push_str("</div>");
            img_path.push(i.url.clone());
        }
        ImageReference(_) => {}
        MdxJsxTextElement(_) => {}
        Link(l) => {
            //println!("{:?}", l);
            output.push_str(
                format!("<a class=\"link\" href=\"{}\" target=\"_blank\">", &l.url).as_str(),
            );
            if let Some(title) = &l.title {
                output.push_str(title);
                *count += words_count::count(title).words;
            }
            for n in &l.children {
                render_markdown(n, output, img_path, count);
            }
            output.push_str("</a>");
        }
        LinkReference(_) => {
            println!("link ref");
        }
        Strong(s) => {
            output.push_str("<strong>");
            for n in &s.children {
                render_markdown(n, output, img_path, count);
            }
            output.push_str("</strong>");
        }
        Code(c) => {
            if let Some(lang) = &c.lang {
                if lang == "youtube" {
                    output.push_str(format!("<iframe class=\"video\" width=\"560\" height=\"315\" src=\"https://www.youtube.com/embed/{}\" title=\"YouTube video player\" frameborder=\"0\" allow=\"accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share\" allowfullscreen></iframe>", &c.value).as_str());
                } else {
                    output.push_str(
                        format!("<pre><code class=\"language-{} code-block\">", lang).as_str(),
                    );
                    output.push_str(&c.value);
                    output.push_str("</code></pre>");
                }
            } else {
                output.push_str("<pre><code>");
                output.push_str(&c.value);
                output.push_str("</code></pre>");
            }
        }
        Math(m) => {
            output.push_str("<code class=\"language-math math-block\">");
            output.push_str(&m.value);
            output.push_str("</code>");
        }
        MdxFlowExpression(_) => {}
        Heading(h) => {
            output.push_str(format!("<h{} >", h.depth).as_str());
            for n in &h.children {
                render_markdown(n, output, img_path, count);
            }
            output.push_str(format!("</h{}>", h.depth).as_str());
        }

        ListItem(li) => {
            output.push_str("<li>");
            for n in &li.children {
                render_markdown(n, output, img_path, count);
            }
            output.push_str("</li>");
        }
        ThematicBreak(tb) => {}
        _ => {
            println!("Unimplemented node. {:?}", &node);
        }
    };
}

fn populate_templates(force: bool) {
    let target_folder = "template";
    let target_folder_exists = Path::new("template").exists();

    if target_folder_exists {
        let target_is_dir: bool = Path::new(target_folder).is_dir();
        if !target_is_dir {
            println!("template {} is not a folder.", target_folder);
            return;
        }
    } else {
        fs::create_dir(target_folder)
            .expect(format!("Unable to create template folder: {}.", target_folder).as_str());
    }

    let files = ["post.html", "index.html", "style.css"];

    for f in files {
        let file_path = format!("{}/{}", target_folder, f);
        let template_exists = Path::new(&file_path).exists();

        if force || !template_exists {
            let content = Template::get(f).unwrap();
            fs::write(&file_path, content.data).unwrap();
        }
    }
}

fn main() {
    let args = Args::parse();
    //https://patorjk.com/software/taag/#p=display&f=Ogre&t=QuipQuick
    println!(
        "         ____       _         ____       _      _    
        /___ \\_   _(_)_ __   /___ \\_   _(_) ___| | __
       //  / / | | | | '_ \\ //  / / | | | |/ __| |/ /
      / \\_/ /| |_| | | |_) / \\_/ /| |_| | | (__|   < 
      \\___,_\\ \\__,_|_| .__/\\___,_\\ \\__,_|_|\\___|_|\\_\\
                     |_|                             "
    );
    let current_time = Local::now();

    populate_templates(true);

    const VERSION: &str = env!("CARGO_PKG_VERSION");

    let contents =
        fs::read_to_string(args.manifest).expect("Should have been able to read the file");

    let value = match contents.parse::<Value>() {
        Err(error) => {
            println!("Toml Parsing Error: {}", error.to_string());
            return;
        }
        Ok(value) => value,
    };

    if let toml::Value::Table(ref global) = value {
        let mut target_folder = args.target.clone();
        if global.contains_key("target") {
            if let Some(target) = global.get("target") {
                if let toml::Value::String(target_str) = target {
                    target_folder = target_str.clone();
                }
            }
        }

        let target_folder_exists = Path::new(&target_folder).exists();

        if target_folder_exists {
            let target_is_dir: bool = Path::new(&target_folder).is_dir();
            if !target_is_dir {
                println!("Target {} is not a folder.", &target_folder);
                return;
            }

            fs::remove_dir_all(&target_folder).unwrap();
        }

        fs::create_dir(&target_folder)
            .expect(format!("Unable to create target folder: {}.", &target_folder).as_str());

        let blog_title = if global.contains_key("title") {
            let mut title = String::new();
            if let Some(title_value) = global.get("title") {
                if let toml::Value::String(title_str) = title_value {
                    title = title_str.clone();
                }
            }
            title
        } else {
            String::new()
        };

        let blog_description = if global.contains_key("description") {
            let mut description = String::new();
            if let Some(description_value) = global.get("description") {
                if let toml::Value::String(description_str) = description_value {
                    description = description_str.clone();
                }
            }
            description
        } else {
            String::new()
        };

        let repo = if global.contains_key("repo") {
            let mut repo = String::new();
            if let Some(repo_value) = global.get("repo") {
                if let toml::Value::String(repo_str) = repo_value {
                    repo = repo_str.clone();
                }
            }
            repo
        } else {
            String::new()
        };

        let content = match global.get("content") {
            None => {
                println!("Warning: No content detected.");
                return;
            }
            Some(content) => content,
        };

        let template = fs::read_to_string("template/post.html")
            .expect("Should have been able to read the file");

        let reg = Handlebars::new();
        // reg.register_helper("md", Box::new(md));

        let mut post_list: Vec<Post> = Vec::new();

        if let toml::Value::Array(ref content) = content {
            for c in content {
                if let toml::Value::Table(ref post) = c {
                    //print!("{:?}", post);

                    let date = post.get("date").unwrap().as_str().unwrap();
                    println!("date {}", date);
                    let d = date.parse::<DateTimeUtc>().unwrap().0;
                    println!("{:?}", d);
                    let description = post.get("description").unwrap().as_str().unwrap();
                    let folder = post.get("folder").unwrap().as_str().unwrap();
                    let title = post.get("title").unwrap().as_str().unwrap();
                    let tags_value = post.get("tags").unwrap().as_array().unwrap();

                    let mut tags = Vec::new();

                    for t in tags_value {
                        tags.push(t.as_str().unwrap().to_string());
                    }

                    let target_folder_exists =
                        Path::new(format!("{}/{}", target_folder, folder).as_str()).exists();

                    if !target_folder_exists {
                        fs::create_dir(format!("{}/{}", target_folder, folder).as_str()).expect(
                            format!("Unable to create target folder: {}.", &folder).as_str(),
                        );
                    }

                    let path = format!("{}/content.md", folder);

                    let markdown =
                        fs::read_to_string(path).expect("Should have been able to read the file");

                    //println!("markdown {}", markdown);

                    let mut options = Options::gfm();
                    options.parse.constructs.math_text = true;
                    options.parse.constructs.frontmatter = true;
                    options.parse.constructs.math_flow = true;

                    let ast = markdown::to_mdast(&markdown, &options.parse).unwrap();

                    //println!("{:?}", ast);
                    let mut rendered_string = String::new();
                    let mut images: Vec<String> = Vec::new();
                    let mut word_count: usize = 0;
                    render_markdown(&ast, &mut rendered_string, &mut images, &mut word_count);
                    println!("word count: {}", word_count);

                    let data = Post {
                        date: d,
                        description: description.to_string(),
                        md: rendered_string,
                        title: title.to_string(),
                        tags: tags,
                        word_count: word_count,
                        blog_title: blog_title.clone(),
                        repo: repo.clone(),
                        quipquick_version: VERSION.to_string(),
                        current_time: format!("{}", current_time.format("%Y-%m-%d %H:%M:%S")),
                    };

                    post_list.push(data.clone());

                    //println!("{} {:?}", rendered_string, images);

                    for i in images {
                        let copy_from = format!("{}/{}", folder, i);
                        let copy_to = format!("{}/{}/{}", target_folder, folder, i);

                        std::fs::copy(copy_from, copy_to).unwrap();
                    }

                    let rendered = reg.render_template(&template, &data).unwrap();

                    let output_path = format!("{}/{}/index.html", target_folder, folder);

                    fs::write(output_path, rendered).unwrap();
                } else {
                    println!("Toml Format Error: A chapter needs to be a table format.");
                }
            }
        }

        post_list.sort_by(|a, b| {
            if a.date < b.date {
                return Ordering::Greater;
            } else if a.date == b.date {
                return Ordering::Equal;
            } else {
                return Ordering::Less;
            }
        });

        let index_template = fs::read_to_string("template/index.html")
            .expect("Should have been able to read the file");

        let data = json!({
            "posts": post_list,
            "repo": repo,
            "blog_title": blog_title,
            "blog_description": blog_description,
            "quipquick_version": VERSION,
            "current_time": format!("{}", current_time.format("%Y-%m-%d %H:%M:%S"))
        });

        let index_rendered = reg.render_template(&index_template, &data).unwrap();

        let output_path = format!("{}/index.html", target_folder);

        fs::write(output_path, index_rendered).unwrap();

        fs::copy("template/style.css", format!("{}/style.css", target_folder)).unwrap();
    }

    /*let mut reg = Handlebars::new();
    handlebars_helper!(date2: |dt: OffsetDateTime, {fmt:str = "[year]-[month]-[day]"}|
        dt.format(&parse(fmt).unwrap()).unwrap()
    );
    reg.register_helper("date2", Box::new(date2));

    // render without register
    println!(
        "{}",
        reg.render_template("Hello {{name}}", &json!({"name": "foo"}))
            .unwrap()
    );*/

    // register template using given name
    /*reg.register_template_string("tpl_1", "Good afternoon, {{name}}")
        .unwrap();
    println!("{}", reg.render("tpl_1", &json!({"name": "foo"})).unwrap());

    let data = OffsetDateTime::now_utc();

    println!(
        "{}",
        reg.render_template("<div>{{date2 this}}</div>", &data)
            .unwrap()
    );*/
}
