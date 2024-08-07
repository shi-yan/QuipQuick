use chrono::Local;
use chrono::{DateTime, Datelike};
use dateparser::parse_with_timezone;
use handlebars::{Handlebars, JsonValue};
use image::io::Reader as ImageReader;
use itertools::Itertools;
use markdown::Options;
use rss::{ChannelBuilder, GuidBuilder, ImageBuilder, Item, ItemBuilder};
use serde_json::json;
use slugify::slugify;
use std::cmp::{self, Ordering};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use toml::Value;

use crate::frontmatter::FrontmatterInfo;
use crate::md2html::{render_markdown, Footnote, SelectedMetaImage};
use crate::new::populate_templates;
use crate::post::{Post, Tag};
use markdown::to_mdast;

pub fn generate_google_analytics_id(id: &str) -> String {
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

pub fn publish(target: String, force_overwrite_theme: bool) {
    let current_time: DateTime<Local> = Local::now();

    populate_templates("./", force_overwrite_theme);

    const VERSION: &str = env!("CARGO_PKG_VERSION");

    let contents =
        fs::read_to_string("quipquick.toml").expect("Should have been able to read the file");

    let value = match contents.parse::<Value>() {
        Err(error) => {
            println!("Toml Parsing Error: {}", error.to_string());
            return;
        }
        Ok(value) => value,
    };

    if let toml::Value::Table(ref global) = value {
        let target_folder = global
            .get("target")
            .and_then(|value| value.as_str())
            .unwrap_or(&target)
            .to_owned();

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
                        println!("Removing {:?} {:?}", item.path(), item.file_name());
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

        let blog_title = global
            .get("title")
            .and_then(|value| value.as_str())
            .expect("Blog title is mandatory")
            .to_owned();

        let blog_description = global
            .get("description")
            .and_then(|value| value.as_str())
            .expect("Blog description is mandatory")
            .to_owned();

        let repo = global
            .get("repo")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_owned();

        let blog_url = global
            .get("url")
            .and_then(|value| value.as_str())
            .expect("Blog url is mandatory!")
            .to_owned();

        let google_analytics_id = global
            .get("google_analytics_id")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_owned();

        let discussion_url = global.get("discussion_url").and_then(|value| {
            Some(
                value
                    .as_str()
                    .expect("Discussion url has to be a string")
                    .to_owned(),
            )
        });

        let logo = global.get("logo").and_then(|value| {
            if let Some(logo_path) = value.as_str() {
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
        });

        let content = global
            .get("content")
            .expect("Warning: No content detected.")
            .as_array()
            .expect("Content needs to be an array.");

        let gallery = global.get("gallery").and_then(|value| value.as_str());

        let template = fs::read_to_string("template/post.html")
            .expect("Should have been able to read the file");

        let reg = Handlebars::new();
        // reg.register_helper("md", Box::new(md));

        let mut post_list: Vec<Post> = Vec::new();

        for c in content {
            let folder = c.as_str().unwrap();

            if folder == "tags" {
                println!("There shouldn't be a content folder named tags");
                return;
            }

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

            let ast = to_mdast(&markdown, &options.parse).unwrap();

            //println!("{:?}", ast);
            let mut rendered_string = String::new();
            let mut word_count: usize = 0;
            let mut frontmatter: FrontmatterInfo = FrontmatterInfo {
                title: String::new(),
                date: String::new(),
                description: String::new(),
                tags: vec![],
            };
            let mut selected_meta_image = logo.clone();
            let mut langs: HashSet<String> = HashSet::new();
            let mut footnotes: HashMap<String, Footnote> = HashMap::new();

            render_markdown(
                &ast,
                &mut rendered_string,
                folder,
                &target_folder,
                &mut word_count,
                &mut frontmatter,
                &mut langs,
                &mut selected_meta_image,
                &mut footnotes,
            );
            if footnotes.len() > 0 {
                rendered_string += "<table class=\"footnote-def\">";
                for key in footnotes.keys().sorted() {
                    let f = footnotes.get(key).unwrap();
                    rendered_string += format!(
                            "<tr class=\"footnote-row\" id=\"footnote_{}\"><td>[{}]: </td><td>{}</td></tr>",
                            f.id, f.count, f.html
                        )
                        .as_str();
                }
                rendered_string += "</table>";
            }

            let d = parse_with_timezone(&frontmatter.date, &chrono::offset::Local).unwrap();
            let mut tags: Vec<Tag> = Vec::new();
            for t in &frontmatter.tags {
                tags.push(Tag {
                    slug: slugify!(t),
                    tag: t.to_lowercase(),
                });
            }

            let mut langs_sorted = Vec::from_iter(langs);
            langs_sorted.sort();

            let data = Post {
                date: d.into(),
                description: frontmatter.description,
                src: folder.to_string(),
                md: rendered_string,
                title: titlecase::titlecase(&frontmatter.title),
                tags: tags,
                word_count: word_count,
                blog_title: blog_title.clone(),
                blog_url: blog_url.clone(),
                repo: repo.clone(),
                quipquick_version: VERSION.to_string(),
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
                langs: langs_sorted,
            };
            post_list.push(data.clone());
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

        const PAGE_ITEM_COUNT: u32 = 5;

        let page_size: u32 = (post_list.len() as f32 / PAGE_ITEM_COUNT as f32).ceil() as u32;

        let mut tags: HashMap<String, (String, Vec<u32>)> = HashMap::new();

        let mut rss_items: Vec<Item> = Vec::new();

        for index in 0..post_list.len() {
            if index > 0 {
                post_list[index].newer_post = Some((
                    titlecase::titlecase(&post_list[index - 1].title),
                    post_list[index - 1].src.clone(),
                ));
            }

            if index < post_list.len() - 1 {
                post_list[index].older_post = Some((
                    titlecase::titlecase(&post_list[index + 1].title),
                    post_list[index + 1].src.clone(),
                ));
            }

            println!(
                "Generating article {} {}",
                format!(
                    "{}-{:0width$}-{:0width$}",
                    &post_list[index].date.year(),
                    &post_list[index].date.month(),
                    &post_list[index].date.day(),
                    width = 2
                )
                .as_str(),
                &post_list[index].title
            );

            let rendered = reg.render_template(&template, &post_list[index]).unwrap();

            let output_path = format!("{}/{}/index.html", target_folder, &post_list[index].src);

            fs::write(output_path, rendered).unwrap();

            for t in &post_list[index].tags {
                if tags.contains_key(&t.slug) {
                    tags.get_mut(&t.slug).unwrap().1.push(index as u32);
                } else {
                    tags.insert(t.slug.clone(), (t.tag.clone(), vec![index as u32]));
                }
            }

            let permanent_link = format!("{}/{}", &blog_url, post_list[index].src);
            let guid = GuidBuilder::default().value(permanent_link.clone()).build();

            let item = ItemBuilder::default()
                .title(Some(titlecase::titlecase(&post_list[index].title)))
                .link(Some(permanent_link))
                .description(Some(post_list[index].description.clone()))
                .comments(post_list[index].discussion_url.clone())
                .guid(Some(guid))
                .build();
            rss_items.push(item)
        }

        let rss_output_path = format!("{}/rss.xml", target_folder);

        let channel = if let Some(l) = &logo {
            let rss_image = ImageBuilder::default()
                .url(format!("{}/{}", &blog_url, l.url))
                .title(blog_title.clone())
                .link(blog_url.clone())
                .build();

            let channel = ChannelBuilder::default()
                .title(blog_title.clone())
                .link(blog_url.clone())
                .description(blog_description.clone())
                .items(rss_items)
                .image(Some(rss_image))
                .build();

            channel
        } else {
            let channel = ChannelBuilder::default()
                .title(blog_title.clone())
                .link(blog_url.clone())
                .description(blog_description.clone())
                .items(rss_items)
                .build();

            channel
        };
        fs::write(rss_output_path, channel.to_string()).unwrap();
        let index_template = fs::read_to_string("template/index.html")
            .expect("Should have been able to read the file");

        for index in 0..page_size {
            let mut pages = Vec::new();

            for pindex in 0..page_size {
                pages.push(json!({"id":pindex+1,"current":index == pindex, "link":if pindex ==0 {String::from("/index.html")} else {format!("/index{}.html",pindex+1)}}));
            }

            let page_range = (index * PAGE_ITEM_COUNT) as usize
                ..cmp::min((index + 1) * PAGE_ITEM_COUNT, post_list.len() as u32) as usize;

            let mut data = json!({
                "posts": post_list[page_range],
                "repo": repo,
                "pages": pages,
                "blog_title": blog_title,
                "blog_description": blog_description,
                "blog_url":blog_url,
                "quipquick_version": VERSION,
                "google_analytics": generate_google_analytics_id(&google_analytics_id),
                "gallery": gallery
            });

            if let Some(logo) = &logo {
                fs::copy(&logo.url, format!("{}/{}", target_folder, logo.url)).unwrap();
                data.as_object_mut()
                    .unwrap()
                    .insert("logo".to_string(), JsonValue::String(logo.url.clone()));
            }

            if index > 0 {
                let prev_path = if index - 1 == 0 {
                    String::from("/index.html")
                } else {
                    format!("/index{}.html", index)
                };
                data.as_object_mut()
                    .unwrap()
                    .insert("prev".to_string(), JsonValue::String(prev_path));
            }

            if index < page_size - 1 {
                let next_path = format!("/index{}.html", index + 2);
                data.as_object_mut()
                    .unwrap()
                    .insert("next".to_string(), JsonValue::String(next_path));
            }

            let index_rendered = reg.render_template(&index_template, &data).unwrap();

            let output_path = if index == 0 {
                format!("{}/index.html", target_folder)
            } else {
                format!("{}/index{}.html", target_folder, index + 1)
            };

            fs::write(output_path, index_rendered).unwrap();
        }

        for t in tags {
            let folder = t.0.as_str();

            let target_folder_exists =
                Path::new(format!("{}/tags/{}", target_folder, folder).as_str()).exists();

            if !target_folder_exists {
                fs::create_dir_all(format!("{}/tags/{}", target_folder, folder).as_str())
                    .expect(format!("Unable to create tag folder: {}.", &folder).as_str());
            }

            let tag_page_size: u32 = (t.1 .1.len() as f32 / PAGE_ITEM_COUNT as f32).ceil() as u32;

            let mut tag_post_list: Vec<Post> = Vec::new();

            for tid in &t.1 .1 {
                tag_post_list.push(post_list[*tid as usize].clone());
            }

            for index in 0..tag_page_size {
                let mut pages = Vec::new();

                for pindex in 0..tag_page_size {
                    pages.push(json!({"id":pindex+1,"current":index == pindex, "link":if pindex ==0 {String::from("/index.html")} else {format!("/index{}.html",pindex+1)}}));
                }

                let page_range = (index * PAGE_ITEM_COUNT) as usize
                    ..cmp::min((index + 1) * PAGE_ITEM_COUNT, t.1 .1.len() as u32) as usize;

                let mut data = json!({
                    "posts": tag_post_list[page_range],
                    "repo": repo,
                    "pages": pages,
                    "blog_title": blog_title,
                    "blog_description": blog_description,
                    "blog_url":blog_url,
                    "quipquick_version": VERSION,
                    "google_analytics": generate_google_analytics_id(&google_analytics_id),
                    "gallery": gallery
                });

                if let Some(logo) = &logo {
                    fs::copy(&logo.url, format!("{}/{}", target_folder, logo.url)).unwrap();
                    data.as_object_mut()
                        .unwrap()
                        .insert("logo".to_string(), JsonValue::String(logo.url.clone()));
                }

                if index > 0 {
                    let prev_path = if index - 1 == 0 {
                        format!("/tags/{}/index.html", t.0)
                    } else {
                        format!("/tags/{}/index{}.html", t.0, index)
                    };
                    data.as_object_mut()
                        .unwrap()
                        .insert("prev".to_string(), JsonValue::String(prev_path));
                }

                if index < tag_page_size - 1 {
                    let next_path = format!("/tags/{}/index{}.html", t.0, index + 2);
                    data.as_object_mut()
                        .unwrap()
                        .insert("next".to_string(), JsonValue::String(next_path));
                }

                data.as_object_mut()
                    .unwrap()
                    .insert("page_tag".to_string(), JsonValue::String(t.1 .0.clone()));

                let index_rendered = reg.render_template(&index_template, &data).unwrap();

                let output_path = if index == 0 {
                    format!("{}/tags/{}/index.html", target_folder, folder)
                } else {
                    format!("{}/tags/{}/index{}.html", target_folder, folder, index + 1)
                };

                fs::write(output_path, index_rendered).unwrap();
            }
        }

        fs::write(
            format!("{}/current_time.txt", target_folder).as_str(),
            format!("{}", current_time.format("%Y-%m-%d %H:%M:%S")),
        )
        .unwrap();
        fs::copy("template/style.css", format!("{}/style.css", target_folder)).unwrap();

        if let Some(g) = gallery {
            crate::gallery::generate_gallery(
                &target_folder,
                &g,
                &repo,
                &blog_title,
                &blog_description,
                &blog_url,
                VERSION,
                &google_analytics_id,
            );
        }
    }
}
