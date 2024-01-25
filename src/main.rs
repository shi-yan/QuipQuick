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
use std::fs::File;
use std::fs::{self, FileType};
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
use image::io::Reader as ImageReader;
use markdown::{Constructs, Options, ParseOptions};
use rust_embed::{EmbeddedFile, RustEmbed};
use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};
use serde::Deserialize;
use words_count::WordsCount;

fn default_as_false() -> bool {
    false
}

#[derive(Deserialize, Debug)]
struct PostInfo {
    title: String,
    #[serde(default)]
    folder: String,
    date: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default = "default_as_false")]
    is_draft: bool,
}

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
    src: String,
    md: String,
    title: String,
    tags: Vec<String>,
    word_count: usize,
    repo: String,
    blog_title: String,
    blog_url: String,
    quipquick_version: String,
    current_time: String,
    google_analytics: String,
    read_time: u32,
    older_post: Option<(String, String)>,
    newer_post: Option<(String, String)>,
    discussion_url: Option<String>,
    meta_img: Option<String>,
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
        map.serialize_entry("src", &self.src).unwrap();
        map.serialize_entry("md", &self.md).unwrap();
        map.serialize_entry("title", &self.title).unwrap();
        map.serialize_entry("tags", &self.tags).unwrap();
        map.serialize_entry("word_count", &self.word_count).unwrap();
        map.serialize_entry("repo", &self.repo).unwrap();
        map.serialize_entry("blog_url", &self.blog_url).unwrap();
        map.serialize_entry("blog_title", &self.blog_title).unwrap();
        map.serialize_entry("quipquick_version", &self.quipquick_version)
            .unwrap();
        map.serialize_entry("current_time", &self.current_time)
            .unwrap();
        map.serialize_entry("google_analytics", &self.google_analytics)
            .unwrap();
        map.serialize_entry("read_time", &self.read_time).unwrap();

        if let Some(newer_post) = &self.newer_post {
            map.serialize_entry("newer_post_title", &newer_post.0)
                .unwrap();
            map.serialize_entry("newer_post_folder", &newer_post.1)
                .unwrap();
        }

        if let Some(older_post) = &self.older_post {
            map.serialize_entry("older_post_title", &older_post.0)
                .unwrap();
            map.serialize_entry("older_post_folder", &older_post.1)
                .unwrap();
        }

        if let Some(discussion_url) = &self.discussion_url {
            map.serialize_entry("discussion_url", discussion_url)
                .unwrap();
        }

        if let Some(mi) = &self.meta_img {
            map.serialize_entry("meta_img", mi).unwrap();
        }

        map.end()
    }
}

fn generate_google_analytics_id(id: &str) -> String {
    return format!(
        "<!-- Google tag (gtag.js) -->\n\
    <script async src=\"https://www.googletagmanager.com/gtag/js?id={}\"></script>\n\
    <script>\n\
      window.dataLayer = window.dataLayer || [];\n\
      function gtag() {{ dataLayer.push(arguments); }}\n\
      gtag('js', new Date());\n\
      gtag('config', '{}');\n\
    </script>",
        id, id
    );
}

#[derive(Debug, Clone)]
struct SelectedMetaImage {
    url: String,
    aspect_ratio: f32,
    pixels: u32,
}

fn render_markdown(
    node: &Node,
    output: &mut String,
    folder: &str,
    target_folder: &str,
    count: &mut usize,
    meta: &mut PostInfo,
    selected_meta_image: &mut Option<SelectedMetaImage>,
) {
    match node {
        Paragraph(p) => {
            output.push_str("<p>");

            for n in node.children().unwrap() {
                render_markdown(
                    n,
                    output,
                    folder,
                    target_folder,
                    count,
                    meta,
                    selected_meta_image,
                );
            }

            output.push_str("</p>");
        }
        Text(t) => {
            *count += words_count::count(&t.value).words;
            output.push_str(&t.value);
        }
        Root(r) => {
            for n in &r.children {
                render_markdown(
                    n,
                    output,
                    folder,
                    target_folder,
                    count,
                    meta,
                    selected_meta_image,
                );
            }
        }
        BlockQuote(b) => {
            output.push_str("<blockquote>");

            for n in &b.children {
                render_markdown(
                    n,
                    output,
                    folder,
                    target_folder,
                    count,
                    meta,
                    selected_meta_image,
                );
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
                render_markdown(
                    n,
                    output,
                    folder,
                    target_folder,
                    count,
                    meta,
                    selected_meta_image,
                );
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
            *meta = serde_yaml::from_str(&c.value).unwrap();
            println!("{:?}", meta);
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
                render_markdown(
                    n,
                    output,
                    folder,
                    target_folder,
                    count,
                    meta,
                    selected_meta_image,
                );
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

            let img = ImageReader::open(format!("{}/{}", folder, i.url))
                .unwrap()
                .decode()
                .unwrap();

            if img.width() > 768 || img.height() > 400 {
                let shrink_ratio = (768.0/img.width() as f32).min( 400.0/img.height() as f32);

                let thumb = img.resize((img.width() as f32 * shrink_ratio) as u32, (img.height() as f32 * shrink_ratio) as u32, image::imageops::FilterType::Lanczos3);
                let thumb_path = format!("{}/{}/thumb_{}", target_folder, folder, i.url);
                thumb.save(&thumb_path).unwrap();
                output.push_str(
                    format!(
                        "<img class=\"img\" onclick=\"openImage(this)\" src=\"thumb_{}\" original_src=\"{}\" alt=\"{}\" />",
                        &i.url,&i.url, &i.alt
                    )
                    .as_str(),
                );

                let pixels: u32 = thumb.width() * thumb.height();
                let aspect_ratio = ((thumb.width() as f32 / thumb.height() as f32) - 1.0).abs();
                if let Some(si) = selected_meta_image {
                    if si.aspect_ratio > aspect_ratio || si.pixels < pixels {
                        *si = SelectedMetaImage {
                            pixels,
                            aspect_ratio,
                            url: format!("{}/thumb_{}", folder, i.url),
                        };
                    }
                } else {
                    *selected_meta_image = Some(SelectedMetaImage {
                        pixels,
                        aspect_ratio,
                        url: format!("{}/thumb_{}", folder, i.url),
                    });
                }
            } else {
                let img_path = format!("{}/{}", folder, i.url);
                output.push_str(
                    format!(
                        "<img class=\"img\" onclick=\"openImage(this)\" src=\"{}\" alt=\"{}\" />",
                        &i.url, &i.alt
                    )
                    .as_str(),
                );

                let pixels: u32 = img.width() * img.height();
                let aspect_ratio = ((img.width() as f32 / img.height() as f32) - 1.0).abs();
                if let Some(si) = selected_meta_image {
                    if si.aspect_ratio > aspect_ratio || si.pixels < pixels {
                        *si = SelectedMetaImage {
                            pixels,
                            aspect_ratio,
                            url: img_path,
                        };
                    }
                } else {
                    *selected_meta_image = Some(SelectedMetaImage {
                        pixels,
                        aspect_ratio,
                        url: img_path,
                    });
                }
            }

            if i.alt.len() > 0 {
                output.push_str("<div class=\"img-title\">");
                output.push_str(&i.alt);
                output.push_str("</div>");
            }
            output.push_str("</div>");

            let copy_from = format!("{}/{}", folder, i.url);
            let copy_to = format!("{}/{}/{}", target_folder, folder, i.url);

            std::fs::copy(copy_from, copy_to).unwrap();
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
                render_markdown(
                    n,
                    output,
                    folder,
                    target_folder,
                    count,
                    meta,
                    selected_meta_image,
                );
            }
            output.push_str("</a>");
        }
        LinkReference(_) => {
            println!("link ref");
        }
        Strong(s) => {
            output.push_str("<strong>");
            for n in &s.children {
                render_markdown(
                    n,
                    output,
                    folder,
                    target_folder,
                    count,
                    meta,
                    selected_meta_image,
                );
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
            output.push_str("<p><code class=\"language-math math-block\">");
            output.push_str(&m.value);
            output.push_str("</code></p>");
        }
        MdxFlowExpression(_) => {}
        Heading(h) => {
            output.push_str(format!("<h{} >", h.depth).as_str());
            for n in &h.children {
                render_markdown(
                    n,
                    output,
                    folder,
                    target_folder,
                    count,
                    meta,
                    selected_meta_image,
                );
            }
            output.push_str(format!("</h{}>", h.depth).as_str());
        }

        ListItem(li) => {
            output.push_str("<li>");
            for n in &li.children {
                render_markdown(
                    n,
                    output,
                    folder,
                    target_folder,
                    count,
                    meta,
                    selected_meta_image,
                );
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

            let items = fs::read_dir(&target_folder).unwrap();

            for path in items {
                if let Ok(item) = path {
                    if !item.file_name().eq_ignore_ascii_case(".git")
                        && !item.file_name().eq_ignore_ascii_case("README.md")
                    {
                        println!("removing {:?} {:?}", item.path(), item.file_name());
                        if let Ok(file_type) = item.file_type() {
                            if file_type.is_dir() {
                                fs::remove_dir_all(item.path()).unwrap();
                            } else {
                                fs::remove_file(item.path()).unwrap();
                            }
                        }
                    }
                }
            }
        } else {
            fs::create_dir(&target_folder)
                .expect(format!("Unable to create target folder: {}.", &target_folder).as_str());
        }

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

        let blog_url = if global.contains_key("url") {
            let mut url = String::new();
            if let Some(url_value) = global.get("url") {
                if let toml::Value::String(url_str) = url_value {
                    url = url_str.clone();
                }
            }
            url
        } else {
            String::new()
        };

        let google_analytics_id = if global.contains_key("google_analytics_id") {
            let mut google_analytics_id = String::new();
            if let Some(google_analytics_id_value) = global.get("google_analytics_id") {
                if let toml::Value::String(google_analytics_id_str) = google_analytics_id_value {
                    google_analytics_id = google_analytics_id_str.clone();
                }
            }
            google_analytics_id
        } else {
            String::new()
        };

        let discussion_url = if global.contains_key("discussion_url") {
            let mut discussion_url = String::new();
            if let Some(discussion_url_value) = global.get("discussion_url") {
                if let toml::Value::String(discussion_url_str) = discussion_url_value {
                    discussion_url = discussion_url_str.clone();
                }
            }
            Some(discussion_url)
        } else {
            None
        };

        let logo = if global.contains_key("logo") {
            if let Some(logo_value) = global.get("logo") {
                if let toml::Value::String(logo_path) = logo_value {
                    if Path::new(logo_path).exists() {
                        let img = ImageReader::open(logo_path).unwrap().decode().unwrap();
                        let aspect_ratio = ((img.width() as f32 / img.height() as f32) - 1.0).abs();
                        Some(SelectedMetaImage {
                            pixels: img.width() * img.height(),
                            aspect_ratio,
                            url: logo_path.to_string(),
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
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
                let folder = c.as_str().unwrap();

                let target_folder_exists =
                    Path::new(format!("{}/{}", target_folder, folder).as_str()).exists();

                if !target_folder_exists {
                    fs::create_dir(format!("{}/{}", target_folder, folder).as_str())
                        .expect(format!("Unable to create target folder: {}.", &folder).as_str());
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
                let mut word_count: usize = 0;
                let mut meta: PostInfo = PostInfo {
                    title: String::new(),
                    folder: String::new(),
                    date: String::new(),
                    description: String::new(),
                    tags: vec![],
                    is_draft: false,
                };
                let mut selected_meta_image = logo.clone();

                render_markdown(
                    &ast,
                    &mut rendered_string,
                    folder,
                    &target_folder,
                    &mut word_count,
                    &mut meta,
                    &mut selected_meta_image,
                );
                if !meta.is_draft {
                    let d = meta.date.parse::<DateTimeUtc>().unwrap().0;

                    let data = Post {
                        date: d,
                        description: meta.description,
                        src: folder.to_string(),
                        md: rendered_string,
                        title: meta.title,
                        tags: meta.tags,
                        word_count: word_count,
                        blog_title: blog_title.clone(),
                        blog_url: blog_url.clone(),
                        repo: repo.clone(),
                        quipquick_version: VERSION.to_string(),
                        current_time: format!("{}", current_time.format("%Y-%m-%d %H:%M:%S")),
                        google_analytics: generate_google_analytics_id(&google_analytics_id),
                        read_time: word_count as u32 / 238,
                        older_post: None,
                        newer_post: None,
                        discussion_url: discussion_url.clone(),
                        meta_img: if let Some(si) = selected_meta_image {
                            Some(si.url)
                        } else {
                            None
                        },
                    };
                    post_list.push(data.clone());
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

        for index in 0..post_list.len() {
            if index > 0 {
                post_list[index].newer_post = Some((
                    post_list[index - 1].title.clone(),
                    post_list[index - 1].src.clone(),
                ));
            }

            if index < post_list.len() - 1 {
                post_list[index].older_post = Some((
                    post_list[index + 1].title.clone(),
                    post_list[index + 1].src.clone(),
                ));
            }

            let rendered = reg.render_template(&template, &post_list[index]).unwrap();

            let output_path = format!("{}/{}/index.html", target_folder, &post_list[index].src);

            fs::write(output_path, rendered).unwrap();
        }

        let index_template = fs::read_to_string("template/index.html")
            .expect("Should have been able to read the file");

        let mut data = json!({
            "posts": post_list,
            "repo": repo,
            "blog_title": blog_title,
            "blog_description": blog_description,
            "blog_url":blog_url,
            "quipquick_version": VERSION,
            "current_time": format!("{}", current_time.format("%Y-%m-%d %H:%M:%S")),
            "google_analytics": generate_google_analytics_id(&google_analytics_id)
        });

        if let Some(logo) = logo {
            fs::copy(&logo.url, format!("{}/{}", target_folder, logo.url)).unwrap();
            data.as_object_mut()
                .unwrap()
                .insert("logo".to_string(), JsonValue::String(logo.url));
        }

        let index_rendered = reg.render_template(&index_template, &data).unwrap();

        let output_path = format!("{}/index.html", target_folder);

        fs::write(output_path, index_rendered).unwrap();
        fs::write(format!("{}/current_time.txt", target_folder).as_str(), format!("{}", current_time.format("%Y-%m-%d %H:%M:%S"))).unwrap();
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
