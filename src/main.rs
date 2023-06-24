mod osrm;

use std::collections::HashMap;

use axum::{
    extract::Multipart,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use geo::{coord, LineString, Point};
use geojson::{Feature, Geometry, Value};
use gpx::Gpx;
use osrm::OsrmApi;
use time::OffsetDateTime;

use crate::osrm::{OsrmError, Tracepoint};

enum Error {
    NonGpxUpload,
    UploadReadError,
    ApiError(OsrmError),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Error::NonGpxUpload => (
                StatusCode::BAD_REQUEST,
                "File upload must be of type 'application/gpx+xml'",
            ),
            Error::UploadReadError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Error reading GPX file body",
            ),
            Error::ApiError(ie) => match ie {
                OsrmError::MapMatchingJsonParseError => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to parse response from map matching service",
                ),
                OsrmError::MapMatchingServiceError => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to get response from map matching service",
                ),
            },
        }
        .into_response()
    }
}

type Result<T> = core::result::Result<T, Error>;

fn get_points_timestamps(file: Gpx) -> (Vec<Point>, Vec<i64>) {
    file.tracks
        .iter()
        .flat_map(|track| {
            track.segments.iter().flat_map(|segment| {
                segment.points.iter().map(|point| {
                    (
                        point.point(),
                        OffsetDateTime::parse(
                            &point.time.unwrap().format().unwrap(),
                            &time::format_description::well_known::Iso8601::DEFAULT,
                        )
                        .unwrap()
                        .unix_timestamp(),
                    )
                })
            })
        })
        .unzip()
}

async fn upload_gpx(mut multipart: Multipart) -> Result<String> {
    let api = OsrmApi::new("http://localhost:5000");
    while let Some(field) = multipart.next_field().await.unwrap() {
        let content_type = field.content_type();
        if let Some("application/gpx+xml") = content_type {
            let name = field.name().unwrap().to_string();
            if let Ok(data) = field.bytes().await {
                if let Ok(gpx_data) = gpx::read(&*data) {
                    let (points, timestamps) = get_points_timestamps(gpx_data);
                    let response = api.get_matches(points, timestamps).await.ok().unwrap();
                    let mut longest = response.matchings[0].legs[0].annotation.nodes.len();
                    let mut segment_matches: HashMap<(u64, u64), Vec<_>> = HashMap::new();
                    for t in &response.tracepoints {
                        if let Some(Tracepoint {
                            matchings_index,
                            waypoint_index,
                            location,
                            ..
                        }) = t
                        {
                            if let Some(leg) = response.matchings[*matchings_index]
                                .legs
                                .get(*waypoint_index)
                            {
                                leg.annotation
                                    .nodes
                                    .as_slice()
                                    .windows(2)
                                    .for_each(|window| {
                                        let k = (window[0], window[1]);
                                        if segment_matches.contains_key(&k) {
                                            segment_matches.get_mut(&k).unwrap().push(location);
                                        } else {
                                            let locations = Vec::new();
                                            segment_matches.insert(k, locations);
                                        }
                                    });
                            }
                        }
                    }
                    println!("Matches for 84778720, 6146786309:",);
                    print_geojson(Value::from(&LineString::new(
                        segment_matches
                            .get(&(84778720, 6146786309))
                            .unwrap()
                            .iter()
                            .map(|(x, y)| coord! { x: *x, y: *y })
                            .collect(),
                    )));
                    //let x = segment_matches
                    //    .get(&(84778720, 6146786309))
                    //    .unwrap()
                    //    .iter()
                    //    .map(|(x, y)| coord! {x: *x, y: *y});
                    for m in &response.matchings {
                        for l in &m.legs {
                            longest = longest.max(l.annotation.nodes.len());
                        }
                    }
                    //let matches = get_way_matches(response);
                    //for m in matches {
                    //    eprintln!(
                    //        "{}:\t\t({:.6}, {:.6})->({:.6}, {:.6})",
                    //        m.id, m.start_point.0, m.start_point.1, m.end_point.0, m.end_point.1
                    //    );
                    //}
                    return Ok(String::from("asdf"));
                }
                return Ok(format!("Length of `{}` is {} bytes", name, data.len()));
            } else {
                return Err(Error::UploadReadError);
            }
        } else {
            return Err(Error::NonGpxUpload);
        }
    }

    Ok("Done".to_string())
}

#[tokio::main]
async fn main() {
    // build our application with a single route
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/upload", post(upload_gpx));

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

fn print_geojson(value: Value) {
    let gj = Feature {
        bbox: None,
        id: None,
        properties: None,
        foreign_members: None,
        geometry: Some(Geometry::new(value)),
    };
    println!("{}", gj.to_string());
}
