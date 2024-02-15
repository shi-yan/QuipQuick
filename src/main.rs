use clap::{Parser, Subcommand};
extern crate fs_extra;
extern crate slugify;

mod frontmatter;
mod post;
mod md2html;
mod new;
mod publish;
use publish::publish;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
#[command(about, long_about = None)]
enum Commands {
    /// Start a new blog.
    New {
        /// You blog's name
        #[arg(short, long)]
        name: Option<String>,

        /// Folder for your blog's raw content
        #[arg(short, long)]
        folder: Option<String>,

        /// Target folder for the generated blog
        #[arg(short, long)]
        target: Option<String>,

        /// Generate the blog boilerplate without showing the prompt
        #[arg(short, long, default_value_t = false)]
        quiet: bool,
    },
    /// Create a new post.
    Write {
        /// Blog title
        title: String,
    },
    /// Generate your blog.
    Pub {
        /// Output directory
        #[arg(short, long, default_value_t = String::from("dist"))]
        target: String,

        /// Manifest file
        #[arg(short, long, default_value_t = String::from("quipquick.toml"))]
        manifest: String,

        /// Blog url prefix
        #[arg(short, long)]
        prefix: Option<String>,
    },
}


fn main() {
    println!(
        "         ____       _         ____       _      _    
        /___ \\_   _(_)_ __   /___ \\_   _(_) ___| | __
       //  / / | | | | '_ \\ //  / / | | | |/ __| |/ /
      / \\_/ /| |_| | | |_) / \\_/ /| |_| | | (__|   < 
      \\___,_\\ \\__,_|_| .__/\\___,_\\ \\__,_|_|\\___|_|\\_\\
                     |_|                             \n"
    );
    //https://patorjk.com/software/taag/#p=display&f=Ogre&t=QuipQuick

    let args = Args::parse();

    match args.command {
        Commands::New {
            name,
            folder,
            target,
            quiet,
        } => {
            new::new_blog(name, folder, target, quiet);
        }
        Commands::Pub {
            target,
            manifest,
            prefix,
        } => {
            publish(manifest, target);
        }
        Commands::Write { title } => {}
    }
}
