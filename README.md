<img src="template_src/logo.png" width="400" height="400" />

After experiencing the frustration of losing blog content on various platforms, I've realized the importance of owning my content data. As a result, I've migrated my blog to GitHub Pages. However, I've encountered issues with popular static blog setups like Hugo + PaperModX, which don't handle math equations well and have complex requirements for image paths.

To address these challenges and to have the freedom to customize my blog extensively, I've developed 'quipquick,' my own static blog engine. For a live deployment of this engine, please [visit my blog](https://shi-yan.github.io/).

## Features
* Full support for inline and block math equations.
* Integration with Google Analytics.
* Ability to easily link to GitHub Discussions.
* Embedding of YouTube videos in blog posts.
* RSS feed.
* Footnote.
* Customizable theme.

## Install

```bash
cargo install --git https://github.com/shi-yan/QuipQuick.git
```

## Usage

To use "quipquick," you'll need to set up two folders. One folder will contain all your raw content, including markdown files, images, and the manifest file `quipquick.toml`. This folder should be version controlled in a private git repo. The second folder will be a target folder to hold the generated HTML pages. This folder will be deployed as GitHub Pages.

1. Create a New Blog Boilerplate
```bash
quipquick new
```
This command will set up a new blog template for you to start writing.

2. Create a new post
```bash
quipquick write
```

3. Publish your blog
```bash
quipquick pub
```
After publishing, the target folder should contain the updated HTML pages ready for deployment. You'll need to push these changes to GitHub to deploy your blog as GitHub Pages.

