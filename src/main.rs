use std::collections::HashSet;
use std::path::Path;
use std::fs;

const REFRESH: bool = true;

#[derive(Clone, Debug, Default)]
struct Frontmatter {
	layout: String,
	title: String,
	author: String,
	categories: Vec<String>,
}

fn parse_frontmatter(node: &markdown::mdast::Node) -> Option<Frontmatter> {
	let root = match node {
		markdown::mdast::Node::Root(root) => root,
		_ => return None,
	};
	let first = root.children.first()?;

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
	year: i32,
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
	let layout_post = include_str!("../layouts/post.html");

	let mut posts: Vec<PostIndex> = Vec::new();
	let mut tags = HashSet::new();
	for entry in fs::read_dir("posts").unwrap() {
		let Ok(entry) = entry else { continue };
		let path = entry.path();

		if path.extension().and_then(|s| s.to_str()) != Some("md") {
			println!("Skipping {:?}, not a markdown file", path);
			continue;
		}

		let file_name = path.file_name().unwrap().to_str().unwrap();
		let Some(FileNameStruct { file_name, day, month, year, slug: _ }) = parse_file_name(file_name) else {
			println!("Skipping {}, draft post", file_name);
			continue;
		};

		let content = fs::read_to_string(&path).unwrap();
		let mdast = markdown::to_mdast(&content, &opts.parse).unwrap();
		let fm = parse_frontmatter(&mdast).unwrap();
		let html = markdown::to_html_with_options(&content, &opts).unwrap();
		let categories_str = fm.categories.join(", ");

		for tag in fm.categories {
			tags.insert(tag);
		}

		let title = &fm.title;
		let author = &fm.author;
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
		if is_still_good(&path, Path::new(&dest_path)) {
			println!("Skipping {}, up to date", file_name);
		}
		else {
			println!("Writing {}.html", file_name);
			fs::write(&dest_path, post_html).unwrap();
		}

		posts.push(PostIndex {
			url: format!("{file_name}.html"),
			title: fm.title,
			sort_key: (-year, -month, -day),
			date_str,
			author: fm.author,
			tags: categories_str,
			year,
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

	// Blog post cards grouped by year
	let mut posts_html = String::new();
	let mut current_year: Option<i32> = None;
	for post in posts {
		if current_year != Some(post.year) {
			// close previous year group
			if current_year.is_some() {
				posts_html.push_str("</div></section>\n");
			}
			// open new year group
			posts_html.push_str(&format!(
				concat!(
					r#"<section class="year-group">"#,
					r#"<h2 class="year-heading">{year}</h2>"#,
					r#"<div class="year-list">"#
				),
				year = post.year
			));
			current_year = Some(post.year);
		}

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
	if current_year.is_some() {
		posts_html.push_str("</div></section>\n");
	}

	let index_html = layout_index
		.replace("<!-- TAG FILTER BUTTONS -->", &tags_html)
		.replace("<!-- POST CARDS -->", &posts_html);

	println!("Writing index.html");
	fs::write("public/index.html", index_html).unwrap();

	// Recusively copy static/ to public/
	copy_recursive(Path::new("static"), Path::new("public")).unwrap();
}

fn copy_recursive(src_path: &Path, dest_path: &Path) -> std::io::Result<()> {
	fs::create_dir_all(dest_path)?;
	for entry in fs::read_dir(src_path)? {
		let entry = entry?;
		let path = entry.path();
		let dest_path = dest_path.join(entry.file_name());
		if path.is_dir() {
			copy_recursive(&path, &dest_path)?;
		}
		else if is_still_good(&path, &dest_path) {
			println!("Skipping {:?}, up to date", dest_path);
			continue;
		}
		else {
			println!("Copying {:?} to {:?}", path, dest_path);
			fs::copy(&path, &dest_path)?;
		}
	}
	Ok(())
}

// Checks if destination file exists and is newer than source file
fn is_still_good(src_file: &Path, dest_file: &Path) -> bool {
	if REFRESH {
		return false;
	}
	let src_metadata = fs::metadata(src_file).unwrap();
	let Ok(dest_metadata) = fs::metadata(dest_file) else {
		return false;
	};
	let src_modified = src_metadata.modified().unwrap();
	let dest_modified = dest_metadata.modified().unwrap();
	return dest_modified >= src_modified;
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn renders_gfm_tables_without_rendering_frontmatter() {
		let content = "\
---
title: \"Table test\"
---

| Name | Value |
| --- | ---: |
| answer | `42` |
";
		let mut opts = markdown::Options::gfm();
		opts.parse.constructs.frontmatter = true;

		let mdast = markdown::to_mdast(content, &opts.parse).unwrap();
		let frontmatter = parse_frontmatter(&mdast).unwrap();
		let html = markdown::to_html_with_options(content, &opts).unwrap();

		assert_eq!(frontmatter.title, "Table test");
		assert!(html.contains("<table>"));
		assert!(html.contains("<code>42</code>"));
		assert!(!html.contains("title:"));
	}
}
