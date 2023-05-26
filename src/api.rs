use serde::Deserialize;

#[derive(Debug)]
pub enum ApiError {
    Network(reqwest::Error),
    Deserialization(serde_json::Error),
}

#[derive(Debug, Deserialize, Clone)]
pub struct Rating {
    average: Option<f32>,
}

/// Loads the image from the provided url
async fn load_image(image_url: &str) -> Option<Vec<u8>> {
    if let Ok(response) = reqwest::get(image_url).await {
        if let Ok(bytes) = response.bytes().await {
            let bytes: Vec<u8> = bytes.into();
            return Some(bytes);
        }
    }
    None
}

#[derive(Debug, Deserialize, Clone)]
pub struct Image {
    #[serde(rename = "original")]
    pub original_image_url: String,
    #[serde(rename = "medium")]
    pub medium_image_url: String,
}

pub mod series_searching {
    use super::*;

    // The series name goes after the equals sign
    const SERIES_SEARCH_ADDRESS: &str = "https://api.tvmaze.com/search/shows?q=";

    #[derive(Debug, Deserialize, Clone)]
    pub struct SeriesSearchResult {
        pub show: Show,
    }

    #[derive(Debug, Deserialize, Clone)]
    pub struct Show {
        pub id: u32,
        pub name: String,
        pub premiered: Option<String>,
        pub genres: Vec<String>,
        pub image: Option<Image>,
    }

    pub async fn search_series(
        series_name: String,
    ) -> Result<Vec<(SeriesSearchResult, Option<Vec<u8>>)>, ApiError> {
        let url = format!("{}{}", SERIES_SEARCH_ADDRESS, series_name);
        // let text = reqwest::get(url).await?.text().await?;

        let response = match reqwest::get(url).await.map(|response| response) {
            Ok(response) => response,
            Err(err) => return Err(ApiError::Network(err)),
        };

        let text = match response.text().await.map(|text| text) {
            Ok(text) => text,
            Err(err) => return Err(ApiError::Network(err)),
        };

        match serde_json::from_str::<Vec<SeriesSearchResult>>(&text) {
            Ok(results) => {
                let mut loaded_results = Vec::with_capacity(results.len());
                for result in results {
                    let image_bytes = if let Some(ref url) = result.show.image {
                        println!("Loading image for {}", result.show.name);
                        load_image(&url.medium_image_url).await
                    } else {
                        None
                    };
                    loaded_results.push((result, image_bytes));
                }
                Ok(loaded_results)
            }
            Err(err) => Err(ApiError::Deserialization(err)),
        }
    }
}

pub mod series_information {
    use super::*;

    // The series id goes after the last slash(append at the end of the string)
    const SERIES_INFORMATION_ADDRESS: &str = "https://api.tvmaze.com/shows/";

    #[derive(Debug, Deserialize, Clone)]
    pub struct SeriesMainInformation {
        pub name: String,
        pub language: String,
        pub genres: Vec<String>,
        pub status: String,
        #[serde(rename = "averageRuntime")]
        pub average_runtime: Option<u32>,
        pub premiered: Option<String>,
        pub ended: Option<String>,
        pub rating: Rating,
        pub network: Option<Network>,
        #[serde(rename = "webChannel")]
        pub web_channel: Option<WebChannel>,
        pub summary: String,
        pub image: Option<Image>,
    }

    #[derive(Debug, Deserialize, Clone)]
    pub struct WebChannel {
        pub name: String,
        #[serde(rename = "officialSite")]
        pub official_site: String,
    }

    #[derive(Debug, Deserialize, Clone)]
    pub struct Network {
        name: String,
        country: Country,
        #[serde(rename = "officialSite")]
        official_site_url: Option<String>,
    }

    #[derive(Debug, Deserialize, Clone)]
    pub struct Country {
        name: String,
    }

    pub async fn get_series_main_info(
        series_id: u32,
    ) -> Result<(SeriesMainInformation, Option<Vec<u8>>), ApiError> {
        let url = format!("{}{}", SERIES_INFORMATION_ADDRESS, series_id);
        // reqwest::get(url).await?.json().await

        let response = match reqwest::get(url).await.map(|response| response) {
            Ok(response) => response,
            Err(err) => return Err(ApiError::Network(err)),
        };

        let text = match response.text().await.map(|text| text) {
            Ok(text) => text,
            Err(err) => return Err(ApiError::Network(err)),
        };

        match serde_json::from_str::<SeriesMainInformation>(&text) {
            Ok(series_info) => {
                let image_bytes = if let Some(image_url) = &series_info.image {
                    load_image(&image_url.original_image_url).await
                } else {
                    None
                };
                return Ok((series_info, image_bytes));
            }
            Err(err) => {
                println!("Deserialization text: \n{}\n", text);
                return Err(ApiError::Deserialization(err));
            }
        }
    }
}

pub mod seasons_list {
    use super::*;

    // replace the word SERIES-ID with the actual series id
    const SEASONS_LIST_ADDRESS: &str = "https://api.tvmaze.com/shows/SERIES-ID/seasons";

    #[derive(Debug, Deserialize)]
    pub struct Season {
        number: u32,
        #[serde(rename = "episodeOrder")]
        episode_order: u32,
        #[serde(rename = "premiereDate")]
        premiere_date: Option<String>,
        #[serde(rename = "endDate")]
        end_date: Option<String>,
    }

    pub async fn get_seasons_list(series_id: u32) -> Result<Vec<Season>, reqwest::Error> {
        let url = SEASONS_LIST_ADDRESS.replace("SERIES-ID", &series_id.to_string());
        reqwest::get(url).await?.json().await
    }
}

pub mod episodes_information {
    use super::*;

    const EPISODE_INFORMATION_ADDRESS: &str =
        "https://api.tvmaze.com/shows/SERIES-ID/episodebynumber?season=SEASON&number=EPISODE";

    #[derive(Debug, Deserialize)]
    pub struct Episode {
        name: String,
        season: u32,
        number: u32,
        runtime: u32,
        airdate: Option<String>,
        airtime: String, // can be empty
        airstamp: String,
        rating: Rating,
        image: Image,
        summary: String,
    }

    pub async fn get_episode_information(
        series_id: u32,
        season: u32,
        episode: u32,
    ) -> Result<Episode, reqwest::Error> {
        let url = EPISODE_INFORMATION_ADDRESS.replace("SERIES-ID", &series_id.to_string());
        let url = url.replace("SEASON", &season.to_string());
        let url = url.replace("EPISODE", &episode.to_string());

        reqwest::get(url).await?.json().await
    }
}
