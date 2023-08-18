#![no_std]
extern crate alloc;

use aidoku::{
	error::{AidokuError, AidokuErrorKind, Result},
	prelude::*,
	std::{defaults::defaults_get, String, Vec, net::Request},
	helpers::uri::encode_uri,
	Chapter, Filter, Listing, Manga, MangaPageResult, Page, MangaStatus, FilterType
};
use alloc::string::ToString;
use core::cmp::Ordering;

static mut KAVITA_API_AUTH: String = String::new();

fn get_kavita_api_url() -> String {
	defaults_get("kavitaAddress").unwrap().as_string().unwrap().to_string().trim_end_matches('/').to_string() + "/api"
}

fn get_kavita_api_key() -> String {
	defaults_get("kavitaAPIKey").unwrap().as_string().unwrap().to_string()
}

fn clear_kavita_api_auth() {
	unsafe { KAVITA_API_AUTH = String::new(); }
}

fn get_kavita_api_auth() -> String {
	if unsafe { KAVITA_API_AUTH.is_empty() } {
		let kavita_api_url = get_kavita_api_url();
		let kavita_api_key = get_kavita_api_key();
		let request_url = format!("{}/Plugin/authenticate?apiKey={}&pluginName=Kavya", kavita_api_url, kavita_api_key);

		let response = Request::post(encode_uri(&request_url)).json().unwrap();
		let auth = response.as_object().unwrap().get("token").as_string().unwrap().to_string();

		unsafe {
			KAVITA_API_AUTH = auth;
		}
	}

	unsafe { format!("Bearer {}", KAVITA_API_AUTH) }
}

#[get_manga_list]
fn get_manga_list(filters: Vec<Filter>, page: i32) -> Result<MangaPageResult> {
	let kavita_api_url = get_kavita_api_url();
	let kavita_api_key = get_kavita_api_key();

	let mut query = String::new();

	// Support of filters is not fully implemented yet
	// Need to migrate /api/Series/v2 API in future
	for filter in filters {
		match filter.kind {
			FilterType::Title => {
				query = filter.value.as_string().unwrap().to_string();
				break;
			},
			_ => continue
		}
	}
	
	let request_url;
	let request_body;

	if query.is_empty() {
		request_url = format!("{}/Series/all?PageNumber={}&PageSize={}", kavita_api_url, page, 40);
		request_body = "{}".to_string();
	} else {
		// Cannot use /api/Search/search?queryString API, seems have an internal error
		request_url = format!("{}/Series?PageNumber={}&PageSize={}", kavita_api_url, page, 40);
		request_body = serde_json::json!({
			"seriesNameQuery": query
		}).to_string();
	};

	let request = Request::post(encode_uri(&request_url))
			.header("Authorization", get_kavita_api_auth().as_str())
			.header("Content-Type", "application/json")
			.body(request_body.as_bytes());

	request.send();
	if request.status_code() != 200 {
		clear_kavita_api_auth();
		return Err(AidokuError{
			reason: AidokuErrorKind::JsonParseError
		});
	}

	let response = request.json().unwrap();
	let mut result = Vec::new();
	for series_object in response.as_array().unwrap() {
		let series = series_object.as_object().unwrap();
		let id = series.get("id").as_int().unwrap().to_string();
		let title = series.get("name").as_string().unwrap().to_string();

		result.push(Manga {
			id: id.clone(),
			cover: format!("{}/image/series-cover?seriesId={}&apiKey={}", kavita_api_url, id, kavita_api_key),
			title: title,
			..Default::default()
		});
	}

	let has_more = result.len() == 40 && query.is_empty();
	Ok(MangaPageResult{
		manga: result,
		has_more: has_more
	})
}

#[get_manga_listing]
fn get_manga_listing(listing: Listing, page: i32) -> Result<MangaPageResult> {
	let kavita_api_url = get_kavita_api_url();
	let kavita_api_key = get_kavita_api_key();

	let mut key_id = "id";
	let mut key_title = "name";
	let request_url = match listing.name.as_str() {
		"On Deck" => format!("{}/Series/on-deck?PageNumber={}&PageSize={}", kavita_api_url, page, 40),
		"Recently Updated" => {
			key_id = "seriesId";
			key_title = "seriesName";
			format!("{}/Series/recently-updated-series", kavita_api_url)
		},
		"Newly Added" => format!("{}/Series/recently-added?PageNumber={}&PageSize={}", kavita_api_url, page, 40),
		"Want To Read" => format!("{}/want-to-read?PageNumber={}&PageSize={}", kavita_api_url, page, 40),
		_ => format!("{}/Series/all?PageNumber={}&PageSize={}", kavita_api_url, page, 40)
	};
	
	let request = Request::post(encode_uri(&request_url))
		.header("Authorization", get_kavita_api_auth().as_str())
		.header("Content-Type", "application/json")
		.body(String::from("{}").as_bytes());
	
	request.send();
	if request.status_code() != 200 {
		clear_kavita_api_auth();
		return Err(AidokuError{
			reason: AidokuErrorKind::JsonParseError
		});
	}

	let response = request.json().unwrap();
	let mut result = Vec::new();
	for series_object in response.as_array().unwrap() {
		let series = series_object.as_object().unwrap();
		let id = series.get(key_id).as_int().unwrap().to_string();
		let title = series.get(key_title).as_string().unwrap().to_string();

		result.push(Manga {
			id: id.clone(),
			cover: format!("{}/image/series-cover?seriesId={}&apiKey={}", kavita_api_url, id, kavita_api_key),
			title: title,
			..Default::default()
		});
	}

	let has_more = match listing.name.as_str() {
		"Recently Updated" => false,
		_ => result.len() == 40
	};
	Ok(MangaPageResult{
		manga: result,
		has_more: has_more
	})
}

#[get_manga_details]
fn get_manga_details(manga_id: String) -> Result<Manga> {
	let kavita_api_url = get_kavita_api_url();
	let kavita_api_key = get_kavita_api_key();

	let series_url = format!("{}/Series/{}", kavita_api_url, manga_id);
	let series_req = Request::get(encode_uri(&series_url))
		.header("Authorization", get_kavita_api_auth().as_str());

	series_req.send();
	if series_req.status_code() != 200 {
		clear_kavita_api_auth();
		return Err(AidokuError{
			reason: AidokuErrorKind::JsonParseError
		});
	}

	let metadata_url = format!("{}/Series/metadata?seriesId={}", kavita_api_url, manga_id);
	let metadata_req = Request::get(encode_uri(&metadata_url))
		.header("Authorization", get_kavita_api_auth().as_str());

	metadata_req.send();
	if metadata_req.status_code() != 200 {
		clear_kavita_api_auth();
		return Err(AidokuError{
			reason: AidokuErrorKind::JsonParseError
		});
	}

	let series_resp = series_req.json().unwrap();
	let metadata_resp = metadata_req.json().unwrap();
	let series = series_resp.as_object().unwrap();
	let metadata = metadata_resp.as_object().unwrap();

	let mut authors = Vec::new();
	for author in metadata.get("pencillers").as_array().unwrap() {
		authors.push(author.as_object().unwrap().get("name").as_string().unwrap().to_string());
	}

	let mut artists = Vec::new();
	for artist in metadata.get("writers").as_array().unwrap() {
		artists.push(artist.as_object().unwrap().get("name").as_string().unwrap().to_string());
	}

	let mut categories = Vec::new();
	for category in metadata.get("genres").as_array().unwrap() {
		categories.push(category.as_object().unwrap().get("title").as_string().unwrap().to_string());
	}
	for category in metadata.get("tags").as_array().unwrap() {
		categories.push(category.as_object().unwrap().get("title").as_string().unwrap().to_string());
	}

	Ok(Manga {
		id: manga_id.clone(),
		cover: format!("{}/image/series-cover?seriesId={}&apiKey={}", kavita_api_url, manga_id, kavita_api_key),
		title: series.get("name").as_string().unwrap().to_string(),
		author: authors.join(", ").to_string(),
		artist: artists.join(", ").to_string(),
		description: metadata.get("summary").as_string().unwrap().to_string(),
		categories: categories,
		status: match metadata.get("publicationStatus").as_int().unwrap() {
			0 => MangaStatus::Ongoing,
			1 => MangaStatus::Hiatus,
			2 => MangaStatus::Completed,
			3 => MangaStatus::Cancelled,
			_ => MangaStatus::Unknown
		},
		..Default::default()
	})
}

#[get_chapter_list]
fn get_chapter_list(manga_id: String) -> Result<Vec<Chapter>> {
	let kavita_api_url = get_kavita_api_url();

	let request_url = format!("{}/Series/volumes?seriesId={}", kavita_api_url, manga_id);
	let request = Request::get(encode_uri(&request_url))
		.header("Authorization", get_kavita_api_auth().as_str());

	request.send();
	if request.status_code() != 200 {
		clear_kavita_api_auth();
		return Err(AidokuError{
			reason: AidokuErrorKind::JsonParseError
		});
	}

	let response = request.json().unwrap();
	let mut result = Vec::new();
	for volume_object in response.as_array().unwrap() {
		let volume = volume_object.as_object().unwrap();
		let volume_number = volume.get("name").as_string().unwrap().to_string().parse::<f32>().unwrap();

		for chapter_object in volume.get("chapters").as_array().unwrap() {
			let chapter = chapter_object.as_object().unwrap();
			let id = chapter.get("id").as_int().unwrap().to_string();
			let title = chapter.get("titleName").as_string().unwrap().to_string();
			let chapter_number = chapter.get("number").as_string().unwrap().to_string().parse::<f32>().unwrap();

			//let chapter_read = chapter.get("pagesRead").as_string().unwrap().to_string().parse::<i32>().unwrap();
			let chapter_pages = chapter.get("pages").as_int().unwrap();
			let chapter_special = chapter.get("isSpecial").as_bool().unwrap();

			let info = if chapter_special {
				format!("{} Page · Specials", chapter_pages)
			} else {
				format!("{} Page", chapter_pages)
			};
			
			result.push(Chapter {
				id: id,
				title: title,
				volume: volume_number,
				chapter: chapter_number,
				scanlator: info,
				..Default::default()
			});
		}
	}

	result.sort_by(|a, b| {
		if a.volume == b.volume {
			if a.chapter == b.chapter {
				return Ordering::Equal;
			} else {
				return a.chapter.partial_cmp(&b.chapter).unwrap();
			}
		} else {
			if a.volume == 0.0 || b.volume == 0.0 {
				return b.volume.partial_cmp(&a.volume).unwrap();
			} else {
				return a.volume.partial_cmp(&b.volume).unwrap();
			}
		}
	});
	result.reverse();

	Ok(result)
}

#[get_page_list]
fn get_page_list(_: String, chapter_id: String) -> Result<Vec<Page>> {
	let kavita_api_url = get_kavita_api_url();
	let kavita_api_key = get_kavita_api_key();
	let request_url = format!("{}/Series/chapter?chapterId={}", kavita_api_url, chapter_id);

	let request = Request::get(encode_uri(&request_url))
		.header("Authorization", get_kavita_api_auth().as_str());

	request.send();
	if request.status_code() != 200 {
		clear_kavita_api_auth();
		return Err(AidokuError{
			reason: AidokuErrorKind::JsonParseError
		});
	}

	let response = request.json().unwrap();
	let page_number = response.as_object().unwrap().get("pages").as_int().unwrap();
	Ok((0..page_number).map(|i| Page {
		index: i as i32 + 1,
		url: format!("{}/Reader/image?chapterId={}&page={}&apiKey={}&extractPdf=true", kavita_api_url, chapter_id, i, kavita_api_key),
		..Default::default()
	}).collect())
}