// CRATES
use actix_web::{get, web, HttpResponse, Result};
use askama::Template;
use chrono::{TimeZone, Utc};

#[path = "utils.rs"]
mod utils;
use utils::{nested_val, request, val, Flair, Params, Post, User};

// STRUCTS
#[derive(Template)]
#[template(path = "user.html", escape = "none")]
struct UserTemplate {
	user: User,
	posts: Vec<Post>,
	sort: String,
}

async fn render(username: String, sort: String) -> Result<HttpResponse> {
	let user: User = user(&username).await;
	let posts: Vec<Post> = posts(username, &sort).await;

	let s = UserTemplate { user: user, posts: posts, sort: sort }.render().unwrap();
	Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

// SERVICES
#[get("/u/{username}")]
async fn page(web::Path(username): web::Path<String>, params: web::Query<Params>) -> Result<HttpResponse> {
	match &params.sort {
		Some(sort) => render(username, sort.to_string()).await,
		None => render(username, "hot".to_string()).await,
	}
}

// USER
async fn user(name: &String) -> User {
	// Build the Reddit JSON API url
	let url: String = format!("https://www.reddit.com/user/{}/about.json", name);

	// Send a request to the url, receive JSON in response
	let res = request(url).await;

	User {
		name: name.to_string(),
		icon: nested_val(&res, "subreddit", "icon_img").await,
		karma: res["data"]["total_karma"].as_i64().unwrap(),
		banner: nested_val(&res, "subreddit", "banner_img").await,
		description: nested_val(&res, "subreddit", "public_description").await,
	}
}

// POSTS
async fn posts(sub: String, sort: &String) -> Vec<Post> {
	// Build the Reddit JSON API url
	let url: String = format!("https://www.reddit.com/u/{}/.json?sort={}", sub, sort);

	// Send a request to the url, receive JSON in response
	let res = request(url).await;

	let post_list = res["data"]["children"].as_array().unwrap();

	let mut posts: Vec<Post> = Vec::new();

	for post in post_list.iter() {
		let img = if val(post, "thumbnail").await.starts_with("https:/") {
			val(post, "thumbnail").await
		} else {
			String::new()
		};
		let unix_time: i64 = post["data"]["created_utc"].as_f64().unwrap().round() as i64;
		let score = post["data"]["score"].as_i64().unwrap();
		let title = val(post, "title").await;

		posts.push(Post {
			title: if title.is_empty() { "Comment".to_string() } else { title },
			community: val(post, "subreddit").await,
			body: String::new(),
			author: val(post, "author").await,
			score: if score > 1000 { format!("{}k", score / 1000) } else { score.to_string() },
			media: img,
			url: val(post, "permalink").await,
			time: Utc.timestamp(unix_time, 0).format("%b %e '%y").to_string(),
			flair: Flair(
				val(post, "link_flair_text").await,
				val(post, "link_flair_background_color").await,
				if val(post, "link_flair_text_color").await == "dark" {
					"black".to_string()
				} else {
					"white".to_string()
				},
			),
		});
	}

	posts
}
