use std::path::{Path, PathBuf};

use rocket::{
    Data, Request, Route, async_trait, figment,
    fs::NamedFile,
    http::{ContentType, Header, Method, Status, ext::IntoOwned, uri::Segments},
    outcome::IntoOutcome,
    response::{self, Redirect, Responder},
    route::{Handler, Outcome},
    tokio::{fs::File, io},
};

/// a copy of [`rocket::fs::FileServer`], but serves (.br) sidecar files if requested.
#[derive(Debug, Clone)]
pub struct BrServer {
    root: PathBuf,
    rank: isize,
}

impl BrServer {
    /// The default rank use by `FileServer` routes.
    const DEFAULT_RANK: isize = 10;

    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref();
        assert!(path.is_dir());
        assert!(path.exists());

        BrServer {
            root: path.into(),
            rank: Self::DEFAULT_RANK,
        }
    }

    pub fn rank(mut self, rank: isize) -> Self {
        self.rank = rank;
        self
    }
}

#[async_trait]
impl Handler for BrServer {
    async fn handle<'r>(&self, req: &'r Request<'_>, data: Data<'r>) -> Outcome<'r> {
        use rocket::http::uri::fmt::Path;

        // Get the segments as a `PathBuf`
        let allow_dotfiles = false;
        let path = req
            .segments::<Segments<'_, Path>>(0..)
            .ok()
            .and_then(|segments| segments.to_path_buf(allow_dotfiles).ok())
            .map(|path| self.root.join(path));

        match path {
            Some(p) if p.is_dir() => {
                // Normalize '/a/b/foo' to '/a/b/foo/'.
                if !req.uri().path().ends_with('/') {
                    let normal = req
                        .uri()
                        .map_path(|p| format!("{p}/"))
                        .expect("adding a trailing slash to a known good path => valid path")
                        .into_owned();

                    return Redirect::permanent(normal)
                        .respond_to(req)
                        .or_forward((data, Status::InternalServerError));
                }

                let wants_br = req
                    .headers()
                    .get_one("Accept-Encoding")
                    .is_some_and(|h| h.contains("br"));
                let index_br = p.join("index.html.br");

                if wants_br && index_br.exists() {
                    tracing::info!("serving .br");
                    let index = BrFile::open(index_br).await;
                    index.respond_to(req).or_forward((data, Status::NotFound))
                } else {
                    let index = NamedFile::open(p.join("index.html")).await;
                    index.respond_to(req).or_forward((data, Status::NotFound))
                }
            }
            Some(p) => {
                let wants_br = req
                    .headers()
                    .get_one("Accept-Encoding")
                    .is_some_and(|h| h.contains("br"));
                let p_br = format!("{}.br", p.to_string_lossy());
                let p_br = PathBuf::from(p_br);

                if wants_br && p_br.exists() {
                    tracing::info!("serving .br");
                    let file = BrFile::open(p_br).await;
                    file.respond_to(req).or_forward((data, Status::NotFound))
                } else {
                    let file = NamedFile::open(p).await;
                    file.respond_to(req).or_forward((data, Status::NotFound))
                }
            }
            None => Outcome::forward(data, Status::NotFound),
        }
    }
}

impl From<BrServer> for Vec<Route> {
    fn from(server: BrServer) -> Self {
        let source = figment::Source::File(server.root.clone());
        let mut route = Route::ranked(server.rank, Method::Get, "/<path..>", server);
        route.name = Some(format!("FileServer: {source}").into());
        vec![route]
    }
}

/// like [`rocket::fs::NamedFile`], but has `Content-Encoding: br`.
struct BrFile(PathBuf, File);

impl BrFile {
    pub async fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = File::open(path.as_ref()).await?;
        Ok(Self(path.as_ref().to_path_buf(), file))
    }
}

impl<'r> Responder<'r, 'static> for BrFile {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
        let mut response = self.1.respond_to(req)?;

        response.set_header(Header::new("Content-Encoding", "br"));

        if let Some(stem) = self.0.file_stem()
            && let Some(ext) = Path::new(stem).extension()
            && let Some(ct) = ContentType::from_extension(&ext.to_string_lossy())
        {
            response.adjoin_header(ct);
        }

        Ok(response)
    }
}
