use std::time::Duration;

use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{Router, RouterBuilderDiscoverExt, page},
    view::view,
};

struct App {
    client: reqwest::Client,
    api_url: String,
}

#[derive(Debug, Deserialize)]
struct NowPlaying {
    origin: String,
    origin_url: Option<String>,
    artist: String,
    title: String,
    album: Option<String>,
    length: Option<i64>,
    paused: bool,
    updated_at: i64,
}

#[derive(Debug, Deserialize)]
struct Listens {
    listens: Vec<Listen>,
}

#[derive(Debug, Deserialize)]
struct Listen {
    listened_at: i64,
    artist: String,
    title: String,
    album: Option<String>,
    duration: Option<i64>,
    origin: String,
    origin_url: Option<String>,
}

async fn fetch_nowplaying(app: &App) -> Result<Option<NowPlaying>> {
    let url = format!("{}/api/v1/nowplaying", app.api_url);
    let resp = app.client.get(&url).send().await?;
    match resp.status() {
        reqwest::StatusCode::NO_CONTENT => Ok(None),
        reqwest::StatusCode::OK => Ok(Some(resp.json().await?)),
        status => Err(anyhow::anyhow!("nowplaying API returned {status}").into()),
    }
}

async fn fetch_listens(app: &App) -> Result<Vec<Listen>> {
    let url = format!("{}/api/v1/listens?limit=20", app.api_url);
    let resp = app.client.get(&url).send().await?;
    let body: Listens = resp.json().await?;
    Ok(body.listens)
}

/// Format a unix timestamp as a JST date-time string.
#[allow(clippy::many_single_char_names)]
fn fmt_jst(unix: i64) -> String {
    const JST: i64 = 9 * 3600;
    let secs = unix + JST;
    let days = secs.div_euclid(86_400);
    let time = secs.rem_euclid(86_400);
    let (h, m, s) = (time / 3600, (time % 3600) / 60, time % 60);
    let (y, mth, d) = civil_from_days(days);
    format!("{y:04}-{mth:02}-{d:02} {h:02}:{m:02}:{s:02}")
}

/// Convert days since 1970-01-01 to a (year, month, day) triple (Howard Hinnant's algorithm).
#[allow(clippy::many_single_char_names, clippy::missing_const_for_fn)]
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[tokio::main]
async fn main() {
    let api_url = std::env::var("NOWPLAYING_API")
        .unwrap_or_else(|_| "https://api-nowplaying.tsukumo.f5.si".to_string());
    let app = App {
        client: reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(8))
            .build()
            .unwrap(),
        api_url,
    };
    topcoat::start(Router::builder().discover().app_context(app).build())
        .await
        .unwrap();
}

#[page("/")]
async fn home(cx: &Cx) -> Result {
    let app = app_context::<App>(cx);
    let nowplaying = fetch_nowplaying(app).await?;
    let listens = fetch_listens(app).await?;
    view! {
        <!DOCTYPE html>
        <html lang="ja">
            <head>
                <meta charset="utf-8" />
                <meta http-equiv="refresh" content="5" />
                <title>"nowplaying"</title>
                topcoat::dev::script()
            </head>
            <body>
                <h1>"Now Playing"</h1>
                if let Some(np) = nowplaying {
                    <dl>
                        <dt>"Track"</dt>
                        <dd>
                            if let Some(url) = np.origin_url.as_ref() {
                                <a href=(url) target="_blank" rel="noopener noreferrer">
                                    (np.artist) " - " (np.title)
                                    if let Some(album) = np.album.as_ref() {
                                        " (" (album) ")"
                                    }
                                </a>
                            } else {
                                (np.artist) " - " (np.title)
                                if let Some(album) = np.album.as_ref() {
                                    " (" (album) ")"
                                }
                            }
                        </dd>
                        <dt>"Origin"</dt> <dd>(np.origin)</dd>
                        <dt>"Status"</dt>
                        <dd>
                            if np.paused {
                                "paused"
                            } else {
                                "playing"
                            }
                        </dd>
                        if let Some(length) = np.length {
                            <dt>"Length"</dt> <dd>(length) "s"</dd>
                        }
                        <dt>"Updated"</dt> <dd>(fmt_jst(np.updated_at))</dd>
                    </dl>
                } else {
                    <p>"Nothing is playing."</p>
                }
                <h2>"Recent"</h2>
                <ul>
                    for l in listens {
                        <li>
                            (fmt_jst(l.listened_at)) " | "
                            if let Some(url) = l.origin_url.as_ref() {
                                <a href=(url) target="_blank" rel="noopener noreferrer">
                                    (l.artist) " - " (l.title)
                                    if let Some(album) = l.album.as_ref() {
                                        " (" (album) ")"
                                    }
                                </a>
                            } else {
                                (l.artist) " - " (l.title)
                                if let Some(album) = l.album.as_ref() {
                                    " (" (album) ")"
                                }
                            }
                            if let Some(duration) = l.duration {
                                " [" (duration) "s]"
                            }
                            " @ " (l.origin)
                        </li>
                    }
                </ul>
            </body>
        </html>
    }
}
