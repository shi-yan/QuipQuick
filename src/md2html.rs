use crate::frontmatter::FrontmatterInfo;
use image::io::Reader as ImageReader;
use markdown::mdast::Node::{
    self, BlockQuote, Break, Code, Delete, Emphasis, FootnoteDefinition, FootnoteReference,
    Heading, Html, Image, ImageReference, InlineCode, InlineMath, Link, LinkReference, List,
    ListItem, Math, MdxFlowExpression, MdxJsxFlowElement, MdxJsxTextElement, MdxTextExpression,
    MdxjsEsm, Paragraph, Root, Strong, Text, ThematicBreak, Toml, Yaml,
};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct SelectedMetaImage {
    pub url: String,
    pub aspect_ratio: f32,
    pub pixels: u32,
}

#[derive(Debug, Clone)]
pub struct Footnote {
    pub id: String,
    pub count: i32,
    pub html: String,
}

pub fn render_markdown(
    node: &Node,
    output: &mut String,
    folder: &str,
    target_folder: &str,
    count: &mut usize,
    frontmatter: &mut FrontmatterInfo,
    langs: &mut HashSet<String>,
    selected_meta_image: &mut Option<SelectedMetaImage>,
    footnotes: &mut HashMap<String, Footnote>,
) {
    match node {
        Paragraph(p) => {
            output.push_str("<p>");
            for n in &p.children {
                render_markdown(
                    n,
                    output,
                    folder,
                    target_folder,
                    count,
                    frontmatter,
                    langs,
                    selected_meta_image,
                    footnotes,
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
                    frontmatter,
                    langs,
                    selected_meta_image,
                    footnotes,
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
                    frontmatter,
                    langs,
                    selected_meta_image,
                    footnotes,
                );
            }

            output.push_str("</blockquote>");
        }

        FootnoteDefinition(f) => {
            let mut footnote_html: String = String::new();

            for n in &f.children {
                render_markdown(
                    n,
                    &mut footnote_html,
                    folder,
                    target_folder,
                    count,
                    frontmatter,
                    langs,
                    selected_meta_image,
                    footnotes,
                );
            }

            if let Some(existing_f) = footnotes.get_mut(&f.identifier) {
                existing_f.html = footnote_html;
            } else {
                let footnote = Footnote {
                    id: f.identifier.clone(),
                    count: footnotes.len() as i32 + 1,
                    html: footnote_html,
                };
                footnotes.insert(f.identifier.clone(), footnote);
            }
        }
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
                    frontmatter,
                    langs,
                    selected_meta_image,
                    footnotes,
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
            *frontmatter = serde_yaml::from_str(&c.value).unwrap();
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
                    frontmatter,
                    langs,
                    selected_meta_image,
                    footnotes,
                );
            }
            output.push_str("</em>");
        }
        MdxTextExpression(_) => {}
        FootnoteReference(f) => {
            let count = if !footnotes.contains_key(&f.identifier) {
                let count = footnotes.len() as i32 + 1;
                let footnote = Footnote {
                    id: f.identifier.clone(),
                    count,
                    html: String::new(),
                };
                footnotes.insert(f.identifier.clone(), footnote);
                count
            } else {
                footnotes.get(&f.identifier).unwrap().count
            };

            output.push_str(
                format!(
                    "<a class=\"footnote-ref\" href=\"#footnote_{}\">[{}]</a>",
                    f.identifier, count
                )
                .as_str(),
            );
        }
        Html(h) => {
            output.push_str(&h.value);
        }
        Image(i) => {
            output.push_str("<div class=\"img-container\">");

            let img = ImageReader::open(format!("{}/{}", folder, i.url))
                .expect(format!("Image {}/{} is not found.", folder, i.url).as_str())
                .decode()
                .unwrap();

            let alt_parts_before_escaping: Vec<&str> = i.alt.split('|').collect();
            let alt_str = if alt_parts_before_escaping.len() > 0 {
                titlecase::titlecase(alt_parts_before_escaping[0])
            } else {
                String::from("")
            };
            let sources: Vec<String> = alt_parts_before_escaping
                .iter()
                .skip(1)
                .map(|&elem| html_escape::encode_text(elem).to_string())
                .collect();

            let mut sources_json = String::new();

            for s in &sources {
                sources_json.push_str(format!("\"{}\",", s).as_str());
            }

            if sources_json.len() > 0 {
                sources_json.remove(sources_json.len() - 1);
            }

            if img.width() > 768 || img.height() > 400 {
                let shrink_ratio = (768.0 / img.width() as f32).min(400.0 / img.height() as f32);

                let thumb = img.resize(
                    (img.width() as f32 * shrink_ratio) as u32,
                    (img.height() as f32 * shrink_ratio) as u32,
                    image::imageops::FilterType::Lanczos3,
                );

                let thumb_path = format!("{}/{}/thumb_{}", target_folder, folder, i.url);
                thumb.save(&thumb_path).unwrap();
                output.push_str(
                    format!(
                        "<img class=\"img\" onclick=\"openImage(this)\" src=\"thumb_{}\" original_src=\"{}\" alt=\"{}\" sources='[{}]' />",
                        &i.url,&i.url,&alt_str,sources_json
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
                        "<img class=\"img\" onclick=\"openImage(this)\" src=\"{}\" alt=\"{}\" sources='[{}]' />",
                        &i.url, &alt_str , sources_json
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

            output.push_str("<div class=\"img-title\">");
            output.push_str(&alt_str);

            for i in 0..sources.len() {
                output.push_str(
                    format!(
                        "<a class=\"img-source\" target=\"_blank\" href=\"{}\">[SOURCE{}]</a>",
                        sources[i],
                        if sources.len() > 1 {
                            format!(" {}", i + 1)
                        } else {
                            String::new()
                        }
                    )
                    .as_str(),
                );
            }
            output.push_str("</div>");
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
                    frontmatter,
                    langs,
                    selected_meta_image,
                    footnotes,
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
                    frontmatter,
                    langs,
                    selected_meta_image,
                    footnotes,
                );
            }
            output.push_str("</strong>");
        }
        Code(c) => {
            if let Some(lang) = &c.lang {
                if lang == "youtube" {
                    output.push_str(format!("<iframe class=\"video\" src=\"https://www.youtube.com/embed/{}\" title=\"YouTube video player\" frameborder=\"0\" allow=\"accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share\" allowfullscreen></iframe>", &c.value).as_str());
                } else {
                    output.push_str(
                        format!("<pre><code class=\"language-{} code-block\">", lang).as_str(),
                    );
                    output.push_str(&c.value);
                    output.push_str("</code></pre>");
                    langs.insert(lang.clone());
                }
            } else {
                output.push_str("<pre><code>");
                output.push_str(&c.value);
                output.push_str("</code></pre>");
            }
        }
        Math(m) => {
            output.push_str(
                "<p class=\"katex-display-counter\"><code class=\"language-math math-block\">",
            );
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
                    frontmatter,
                    langs,
                    selected_meta_image,
                    footnotes,
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
                    frontmatter,
                    langs,
                    selected_meta_image,
                    footnotes,
                );
            }
            output.push_str("</li>");
        }
        ThematicBreak(_tb) => {}
        _ => {
            println!("Unimplemented node. {:?}", &node);
        }
    };
}
