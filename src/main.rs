use axum::{
    extract::Multipart,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use geo::Point;
use gpx::Gpx;
use itertools::Itertools;
use time::OffsetDateTime;

enum Error {
    NonGpxUpload,
    UploadReadError,
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

struct OsrmApi {
    domain: String,
}

impl OsrmApi {
    fn get_osrm_match_query(&self, points: Vec<Point>, timestamps: Vec<i64>) -> String {
        format!(
            "{}/match/v1/foot/{}?timestamps={}",
            self.domain,
            points
                .iter()
                .map(|p| format!("{},{}", p.x(), p.y()))
                .intersperse(";".to_string())
                .collect::<String>(),
            timestamps
                .iter()
                .map(|ts| ts.to_string())
                .intersperse(";".to_string())
                .collect::<String>()
        )
    }
}

async fn upload_gpx(mut multipart: Multipart) -> Result<String> {
    while let Some(field) = multipart.next_field().await.unwrap() {
        let content_type = field.content_type();
        if let Some("application/gpx+xml") = content_type {
            let name = field.name().unwrap().to_string();
            if let Ok(data) = field.bytes().await {
                if let Ok(gpx_data) = gpx::read(&*data) {
                    let (points, timestamps) = get_points_timestamps(gpx_data);
                    let api = OsrmApi {
                        domain: "http://localhost:1234".to_string(),
                    };
                    println!("{}", api.get_osrm_match_query(points, timestamps));
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
