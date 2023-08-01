mod osrm;

use std::{collections::HashMap, fs::File, sync::Arc};

use axum::{
    extract::{multipart::Field, DefaultBodyLimit, Multipart, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use geo::{coord, EuclideanDistance, LineString, Point};
use geo_types::{line_string, point, Coord};
use geojson::{Feature, FeatureCollection, Geometry, Value};
use gpx::Gpx;
use itertools::Itertools;
use osm_xml::{Node, UnresolvedReference, OSM};
use osrm::{OsrmApi, Segment, SegmentMatches};
use time::OffsetDateTime;

use crate::osrm::OsrmError;

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

async fn get_gpx_upload<'a>(field: Field<'a>) -> Result<Gpx> {
    let content_type = field.content_type();
    if let Some("application/gpx+xml") = content_type {
        match field.bytes().await {
            Ok(data) => {
                eprintln!("uhh");
                gpx::read(data.as_ref()).map_err(|_| Error::UploadReadError)
            }
            Err(b) => {
                eprintln!("huh, {}", b.body_text());
                return Err(Error::UploadReadError);
            }
        }
    } else {
        return Err(Error::NonGpxUpload);
    }
}

async fn upload_gpx(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<String> {
    let api = OsrmApi::new("http://localhost:5000");
    while let Some(field) = multipart.next_field().await.unwrap() {
        let gpx_data = get_gpx_upload(field).await?;
        let (points, timestamps) = get_points_timestamps(gpx_data);
        let response = api
            .get_matches(points, timestamps)
            .await
            .map_err(|e| Error::ApiError(e))?;
        let mut segment_matches = response.get_segment_matches();
        let complete_segments = segment_matches.get_complete_segments(&state.nodes);
        let gj = FeatureCollection {
            bbox: None,
            features: complete_segments
                .iter()
                .map(|(s, e)| Feature {
                    geometry: Some(Geometry::new(Value::from(&LineString::new(vec![
                        state.nodes.get(s).unwrap().clone(),
                        state.nodes.get(e).unwrap().clone(),
                    ])))),
                    id: None,
                    bbox: None,
                    properties: None,
                    foreign_members: None,
                })
                .collect(),
            foreign_members: None,
        };
        return Ok(gj.to_string());
    }

    Ok("Done".to_string())
}

const URL: &str = "0.0.0.0:3000";

struct AppState {
    nodes: HashMap<u64, Coord>,
}

#[tokio::main]
async fn main() {
    //let s = "http://localhost:5000/".to_string() + &"testtest".repeat(10000);
    //print!("{}", reqwest::get(s).await.unwrap().text().await.unwrap());
    println!("Reading OSM data...");
    let osm_data = OSM::parse(File::open("provo.xml").unwrap()).unwrap();
    let mut nodes: HashMap<u64, Coord> = HashMap::new();
    for node in osm_data.nodes.values() {
        nodes.insert(node.id as u64, coord! { x: node.lon, y: node.lat });
    }
    for (id, point) in &nodes {
        println!("{}: {:?}", id, point);
        break;
    }
    let shared_state = Arc::new(AppState { nodes });

    // build our application with a single route
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/upload", post(upload_gpx))
        .with_state(shared_state)
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024));

    // run it with hyper on localhost:3000
    println!("Listening on {}", URL);
    axum::Server::bind(&URL.parse().unwrap())
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
