#![no_std]
extern crate alloc;
use aidoku::{
	error::Result,
	prelude::*,
	std::{defaults::defaults_get, String, Vec, net::Request},
	helpers::uri::encode_uri,
	Chapter, Filter, Listing, Manga, MangaPageResult, Page
};
use alloc::string::ToString;
use core::cmp::Ordering;

static mut KAVITA_API_AUTH: String = String::new();
static mut KAVITA_API_AUTH_MUTEX: bool = false;

fn get_kavita_api_url() -> String {
	defaults_get("kavitaAddress").unwrap().as_string().unwrap().to_string().trim_end_matches('/').to_string() + "/api"
}

fn get_kavita_api_key() -> String {
	defaults_get("kavitaAPIKey").unwrap().as_string().unwrap().to_string()
}

fn get_kavita_api_auth() -> String {
	if unsafe { KAVITA_API_AUTH.clone() }.is_empty() {
		unsafe {
			KAVITA_API_AUTH_MUTEX = true;
		}

		// todo!("Implement mutex");

		let kavita_api_url = get_kavita_api_url();
		let kavita_api_key = get_kavita_api_key();
		let request_url = format!("{}/Plugin/authenticate?apiKey={}&pluginName=Kavya", kavita_api_url.clone(), kavita_api_key.clone());

		let response = Request::post(encode_uri(&request_url)).json().unwrap();
		let auth = response.as_object().unwrap().get("token").as_string().unwrap().to_string();

		unsafe {
			KAVITA_API_AUTH = format!("Bearer {}", auth);
			KAVITA_API_AUTH_MUTEX = false;
		}
	}

	unsafe { KAVITA_API_AUTH.clone() }
}

#[get_manga_list]
fn get_manga_list(_filters: Vec<Filter>, page: i32) -> Result<MangaPageResult> {
	let kavita_api_url = get_kavita_api_url();
	let kavita_api_key = get_kavita_api_key();
	let request_url = format!("{}/Series/all?PageNumber={}&PageSize={}", kavita_api_url.clone(), page, 40);

	let response = Request::post(encode_uri(&request_url))
		.header("Authorization", get_kavita_api_auth().as_str())
		.header("Content-Type", "application/json")
		.body(String::from("{}").as_bytes())
		.json()
		.unwrap();

	let mut result = Vec::new();
	for series_object in response.as_array().unwrap() {
		let series = series_object.as_object().unwrap();
		let id = series.clone().get("id").as_int().unwrap().to_string();
		let title = series.get("name").as_string().unwrap().to_string();

		result.push(Manga {
			id: id.clone(),
			cover: format!("{}/image/series-cover?seriesId={}&apiKey={}", kavita_api_url.clone(), id, kavita_api_key.clone()),
			title: title,
			..Default::default()
		});
	}

	let has_more = result.len() == 40;
	Ok(MangaPageResult{
		manga: result,
		has_more: has_more
	})
}

#[get_manga_listing]
fn get_manga_listing(_listing: Listing, page: i32) -> Result<MangaPageResult> {
	let kavita_api_url = get_kavita_api_url();
	let kavita_api_key = get_kavita_api_key();
	let request_url = format!("{}/Series/all?PageNumber={}&PageSize={}", kavita_api_url.clone(), page, 40);

	let response = Request::get(encode_uri(&request_url))
		.header("Authorization", get_kavita_api_auth().as_str())
		.json()
		.unwrap();

	let mut result = Vec::new();
	for series_object in response.as_array().unwrap() {
		let series = series_object.as_object().unwrap();
		let id = series.clone().get("id").as_int().unwrap().to_string();
		let title = series.get("name").as_string().unwrap().to_string();

		result.push(Manga {
			id: id.clone(),
			cover: format!("{}/image/series-cover?seriesId={}&apiKey={}", kavita_api_url.clone(), id, kavita_api_key.clone()),
			title: title,
			..Default::default()
		});
	}

	let has_more = result.len() == 40;
	Ok(MangaPageResult{
		manga: result,
		has_more: has_more
	})
}

#[get_manga_details]
fn get_manga_details(manga_id: String) -> Result<Manga> {
	let kavita_api_url = get_kavita_api_url();
	let kavita_api_key = get_kavita_api_key();
	let request_url = format!("{}/Series/{}", kavita_api_url.clone(), manga_id.clone());

	let request = Request::get(encode_uri(&request_url))
		.header("Authorization", get_kavita_api_auth().as_str())
		.json()
		.unwrap();

	let manga = request.as_object().unwrap();
	Ok(Manga {
		id: manga_id.clone(),
		cover: format!("{}/image/series-cover?seriesId={}&apiKey={}", kavita_api_url.clone(), manga_id.clone(), kavita_api_key.clone()),
		title: manga.get("name").as_string().unwrap().to_string(),
		..Default::default()
	})
}

#[get_chapter_list]
fn get_chapter_list(manga_id: String) -> Result<Vec<Chapter>> {
	let kavita_api_url = get_kavita_api_url();
	let request_url = format!("{}/Series/volumes?seriesId={}", kavita_api_url.clone(), manga_id.clone());

	let response = Request::get(encode_uri(&request_url))
		.header("Authorization", get_kavita_api_auth().as_str())
		.json()
		.unwrap();

	let mut result = Vec::new();

	for volume_object in response.as_array().unwrap() {
		let volume = volume_object.as_object().unwrap();
		let volume_number = volume.clone().get("name").as_string().unwrap().to_string().parse::<f32>().unwrap();

		for chapter_object in volume.get("chapters").as_array().unwrap() {
			let chapter = chapter_object.as_object().unwrap();
			let id = chapter.clone().get("id").as_int().unwrap().to_string();
			let title = chapter.clone().get("titleName").as_string().unwrap().to_string();
			let chapter_number = chapter.get("number").as_string().unwrap().to_string().parse::<f32>().unwrap();
			
			result.push(Chapter {
				id: id,
				title: title,
				volume: volume_number,
				chapter: chapter_number,
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

	Ok(result)
}

#[get_page_list]
fn get_page_list(_: String, chapter_id: String) -> Result<Vec<Page>> {
	let kavita_api_url = get_kavita_api_url();
	let kavita_api_key = get_kavita_api_key();
	let request_url = format!("{}/Series/chapter?chapterId={}", kavita_api_url.clone(), chapter_id.clone());

	let response = Request::get(encode_uri(&request_url))
		.header("Authorization", get_kavita_api_auth().as_str())
		.json()
		.unwrap();

	let page_number = response.as_object().unwrap().get("pages").as_int().unwrap();
	Ok((1..page_number).map(|i| Page {
		index: i as i32,
		url: format!("{}/Reader/image?chapterId={}&page={}&apiKey={}&extractPdf=true", kavita_api_url.clone(), chapter_id, i, kavita_api_key.clone()),
		..Default::default()
	}).collect())
}