use std::collections::HashSet;
use std::fs;

#[derive(Clone, Debug, Default)]
struct Frontmatter {
	layout: String,
	title: String,
	author: String,
	categories: Vec<String>,
}

fn parse_frontmatter(node: &mut markdown::mdast::Node) -> Option<Frontmatter> {
	let root = match node {
		markdown::mdast::Node::Root(root) => root,
		_ => return None,
	};
	let first = root.children.first_mut()?;

	let yaml = match first {
		markdown::mdast::Node::Yaml(yaml) => yaml,
		_ => return None,
	};

	let mut frontmatter = Frontmatter::default();
	for line in yaml.value.lines() {
		if let Some((key, value)) = line.split_once(':') {
			let key = key.trim();
			let value = value.trim();
			match key {
				"layout" => frontmatter.layout = value.to_string(),
				"title" => frontmatter.title = value.trim_matches('"').to_string(),
				"author" => frontmatter.author = value.trim_matches('"').to_string(),
				"categories" => {
					let categories: Vec<String> = value
						.trim_matches(&['[', ']'][..])
						.split(',')
						.map(|s| s.trim().to_string())
						.collect();
					frontmatter.categories = categories;
				}
				_ => {}
			}
		}
	}

	root.children.remove(0);

	Some(frontmatter)
}

struct FileNameStruct<'a> {
	file_name: &'a str,
	year: i32,
	month: i16,
	day: i16,
	slug: &'a str,
}

fn parse_file_name(file_name: &'_ str) -> Option<FileNameStruct<'_>> {
	let file_name = file_name.trim_end_matches(".md");
	let parts: Vec<&str> = file_name.splitn(4, '-').collect();
	if parts.len() < 4 {
		return None;
	}
	let year = parts[0].parse().ok()?;
	let month = parts[1].parse().ok()?;
	let day = parts[2].parse().ok()?;
	let slug = parts[3];
	Some(FileNameStruct { file_name, year, month, day, slug })
}

struct PostIndex {
	url: String,
	title: String,
	sort_key: (i32, i16, i16), // (year, month, day)
	date_str: String,
	author: String,
	tags: String,
}

fn main() {
	// Trusted markdown options
	let mut opts = markdown::Options::gfm();
	opts.parse.constructs.frontmatter = true;
	opts.parse.constructs.html_flow = true;
	opts.parse.constructs.html_text = true;
	opts.compile.allow_dangerous_html = true;
	opts.compile.allow_any_img_src = true;
	opts.compile.allow_dangerous_protocol = true;
	opts.compile.gfm_tagfilter = false;
	let compile = mdast_util_to_markdown::Options::default();

	let layout_post = include_str!("../layouts/post.html");

	let mut posts: Vec<PostIndex> = Vec::new();
	let mut tags = HashSet::new();
	for entry in fs::read_dir("posts").unwrap() {
		let Ok(entry) = entry else { continue };
		let path = entry.path();

		if path.extension().and_then(|s| s.to_str()) != Some("md") {
			continue;
		}

		let file_name = path.file_name().unwrap().to_str().unwrap();
		let Some(file_info) = parse_file_name(file_name) else {
			continue;
		};

		let content = fs::read_to_string(&path).unwrap();
		let mut mdast = markdown::to_mdast(&content, &opts.parse).unwrap();
		let fm = parse_frontmatter(&mut mdast).unwrap();
		let markdown = mdast_util_to_markdown::to_markdown_with_options(&mdast, &compile).unwrap();
		let html = markdown::to_html_with_options(&markdown, &opts).unwrap();
		let categories_str = fm.categories.join(", ");

		for tag in fm.categories {
			tags.insert(tag);
		}

		let title = &fm.title;
		let author = &fm.author;
		let FileNameStruct { file_name, day, month, year, slug: _ } = file_info;
		let month_str = match month {
			1 => "Jan",
			2 => "Feb",
			3 => "Mar",
			4 => "Apr",
			5 => "May",
			6 => "June",
			7 => "July",
			8 => "Aug",
			9 => "Sept",
			10 => "Oct",
			11 => "Nov",
			12 => "Dec",
			_ => "Unknown",
		};
		let date_str = format!("{month_str} {day}, {year}");

		let article = format!("
<article>
  <h1>{title}</h1>
  <div class=\"meta\"><span class=\"date\">{date_str}</span> — <span class=\"author\">by {author}</span> — <span class=\"tags-inline\">{categories_str}</span></div>
{html}
</article>");

		let title_str = format!("<title>Casper's Blog – {}</title>", title);
		let year_author = format!("© {year} {author}");

		let post_html = layout_post
			.replace("<!-- POST CONTENT -->", &article)
			.replace("<!-- POST TITLE -->", &title_str)
			.replace("<!-- YEAR AUTHOR -->", &year_author);

		let dest_path = format!("public/{file_name}.html");
		println!("Writing {}.html", file_name);
		fs::write(&dest_path, post_html).unwrap();

		posts.push(PostIndex {
			url: format!("{file_name}.html"),
			title: fm.title,
			sort_key: (-year, -month, -day),
			date_str,
			author: fm.author,
			tags: categories_str,
		});
	}

	let mut tags: Vec<&String> = tags.iter().collect();
	tags.sort();
	posts.sort_by_key(|post| post.sort_key);

	let layout_index = include_str!("../layouts/index.html");

	// Tag filter buttons
	let mut tags_html = String::new();
	for tag in tags {
		tags_html.push_str(&format!("<button class=\"tag-filter-btn\" data-tag=\"{tag}\">{tag}</button>\n"));
	}

	// Blog post cards
	let mut posts_html = String::new();
	for post in posts {
		let post_card = format!(
			concat!(
				r#"<article class="post-card" data-tags="{tags}">"#,
				r#"<h2><a href="{url}">{title}</a></h2>"#,
				r#"<div class="meta-line">"#,
				r#"<span class="date">{date}</span> — "#,
				r#"<span class="author">{author}</span> — "#,
				r#"<span class="tags-inline">{tags}</span>"#,
				r#"</div>"#,
				r#"</article>"#,
				"\n"
			),
			tags = post.tags,
			url = post.url,
			title = post.title,
			date = post.date_str,
			author = post.author,
		);

		posts_html.push_str(&post_card);
	}

	let index_html = layout_index
		.replace("<!-- TAG FILTER BUTTONS -->", &tags_html)
		.replace("<!-- POST CARDS -->", &posts_html);

	println!("Writing index.html");
	fs::write("public/index.html", index_html).unwrap();
}
