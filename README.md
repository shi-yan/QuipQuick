<img src="template_src/logo.png" width="400" height="400" />

After experiencing the frustration of losing blog content on various platforms, I've realized the importance of owning my content data. As a result, I've migrated my blog to GitHub Pages. However, I've encountered issues with popular static blog setups like [Hugo + PaperModX](https://github.com/reorx/hugo-PaperModX), which don't handle math equations well and have complex requirements for image paths.

To address these challenges and to have the freedom to customize my blog extensively, I've developed QuipQuick, my own static blog engine. For a live deployment of this engine, please [visit my blog](https://shi-yan.github.io/).

## Features
* Full support for inline and block math equations.
* Integration with Google Analytics.
* Ability to easily link to GitHub Discussions.
* Embedding of YouTube videos in blog posts.
* RSS feed.
* Footnote.
* Customizable theme.
* Code block with syntax highlighting.
* Images are resized for quick loading.

## Install

```bash
cargo install --git https://github.com/shi-yan/QuipQuick.git
```

## Usage

To use QuipQuick, you'll need to set up two folders. One folder will contain all your raw content, including markdown files, images, and the manifest file `quipquick.toml`. This folder should be version controlled in a private git repo. The second folder will be a target folder to hold the generated HTML pages. This folder will be deployed as GitHub Pages.

1. Create a New Blog Boilerplate
```bash
quipquick new
```
This command will set up a new blog template for you to start writing. Here is the folder structure:

* `quipquick.toml` This is the manifest file, it contains the global settings for your blog and a list of content folders. Each content folder contains the content (markdown and images) of a single blog post.
* `template` This is the template folder, feel free to modify what's inside to update the theme.
* `dummy_post` This is an example of a content folder. Within this folder, there should be a `content.md` file for the markdown content and images used by the markdown. In the manifest file `quipquick.toml`, the content array should contain the content folder names. If a content folder is not included in the array, the post is considered a draft and won't be published.
* `logo.png` This is a logo image. This image will be used as the icon when you share your blog on social media or for the RSS feed.

2. Create a new post
```bash
quipquick write
```
This command generates a content folder based on the title of your post and update the manifest file.

1. Publish your blog
```bash
quipquick pub
```
After publishing, the target folder should contain the updated HTML pages ready for deployment. You'll need to push these changes to GitHub to deploy your blog as GitHub Pages.

## Customize theme

After the blog boilerplate has been generate using the `new` command, there will be a `template` folder. Within the folder, you can find two html templates and one stylesheet. The template are written in the [handlebars](https://handlebarsjs.com/) template syntax. You can modify these files to change the theme.