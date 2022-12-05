use data_encoding::HEXLOWER;
use rocket::{
    form::{Form, FromForm},
    fs::{relative, FileServer, NamedFile, TempFile},
    http::{ContentType, Method, Status},
    request::{FromRequest, Outcome},
    Request,
};
use secp256k1::{
    schnorr::Signature, Message, Secp256k1, XOnlyPublicKey,
};
use serde::{
    Deserialize, Serialize,
};
use serde_json::{from_str, json, Value};
// use serde_with;
use sha2::{Digest, Sha256};
use std::{
    // collections::HashMap,
    ffi::OsStr,
    fs::{File, read_to_string},
    io,
    io::BufReader,
    path::{Path, PathBuf},
    str,
    // str::FromStr,
    time::SystemTime,
};

#[macro_use]
extern crate rocket;

#[derive(Debug, FromFormField)]
enum Folder {
    All,
    Images,
    Videos,
}
#[derive(Debug, FromFormField)]
enum Order {
    Asc,
    Desc,
}
#[derive(Debug, FromFormField)]
enum TagLogic {
    And,
    Or,
}
// #[derive(Debug, FromForm)]
// struct Tag<'r> {
//     tag: &'r str,
// }
// #[derive(Debug, FromForm)]
// struct Query<'r> {
//     // tags: &'r str,
//     // folder: Folder,
//     folder: &'r str,
//     // #[field(validate = range(1..100))]
//     // limit: u32,
//     // page: u32,
//     since: u32,
//     // until: u32,
//     // order: Order,
// }

#[derive(FromForm)]
struct Query {
    #[field(default = Vec::new())]
    tag: Vec<String>,
    #[field(default = TagLogic::And)]
    tag_logic: TagLogic,
    #[field(default = Folder::All)]
    folder: Folder,
    #[field(default = 0)]
    since: u64,
    #[field(default = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs())]
    until: u64,
    #[field(validate = range(1..100), default = 20)]
    limit: u32,
    #[field(default = 1)]
    page: u32,
    #[field(default = Order::Asc)]
    order: Order,
}

#[derive(Debug, FromForm)]
struct Uploads<'r> {
    upload: Vec<TempFile<'r>>,
}

#[derive(Debug)]
// struct Nwt<'r>(&'r str);
struct Nwt(String);

#[derive(Debug)]
enum NwtError {
    Missing,
    Invalid,
}

// #[derive(Debug, Serialize)]
// enum NwtField<'r> {
//     Id(&'r str),
//     Pubkey(&'r str),
//     Created_at(u64),
//     Kind(u32),
//     Tags(Vec<Vec<&'r str>>),
//     Content(&'r str),
//     Sig(&'r str),
//     Zero,
// }

#[derive(Debug, Serialize, Deserialize)]
struct NwtContent {
    method: Method,
    uri: String,
}

// impl Serialize for NwtContent {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: serde::Serializer,
//     {
//         let mut content = serializer.serialize_struct("NwtEvent", 2)?;
//         content.serialize_field("uri", &self.uri)?;
//         content.serialize_field("method", &self.method)?;
//         content.end()
//     }
// }

// fn serialize_string<S>(x: &str, s: S) -> Result<S::Ok, S::Error>
// where
//     S: Serializer,
// {
//     s.serialize_str(x)
// }

#[derive(Debug, Serialize, Deserialize)]
struct NwtEvent {
    id: String,
    pubkey: String,
    created_at: u64,
    kind: u32,
    // #[serde(skip)]
    tags: Vec<Vec<String>>,
    #[serde(with = "serde_with::json::nested")]
    content: NwtContent,
    sig: String,
    #[serde(skip)]
    username: String,
}

impl NwtEvent {
    fn generate_id(&self) -> String {
        let json = self.serialize_event();
        // sha256::Hash::hash(&json.as_bytes()).to_string()
        sha256_string(json.as_str()).unwrap()
    }
    fn serialize_event(&self) -> String {
        let q = r#"""#;
        str::replace(format!(
            "[0,{},{},{},{:?},{:?}]",
            format!("{}{}{}", q, self.pubkey, q).to_string(),
            self.created_at,
            self.kind,
            self.tags,
            json!(self.content).to_string(),
        ).as_str(), ", ", ",")
    }
}

// frontend
#[get("/")]
async fn index() -> Option<NamedFile> {
    NamedFile::open("app/index.html").await.ok()
}

#[get("/app/<filename>")]
async fn index_resources(filename: &str) -> Option<NamedFile> {
    let path = Path::new(relative!("app")).join(filename);

    NamedFile::open(path).await.ok()
}

#[get("/.well-known/nostr.json")]
async fn nip05() -> Option<NamedFile> {
    NamedFile::open("static/users.json").await.ok()
}

#[get("/<user>")]
async fn user_index(user: &str) -> Option<NamedFile> {
    NamedFile::open("app/index.html").await.ok()
}

#[get("/<user>/<folder>")]
async fn folder_index(user: &str, folder: &str) -> Option<NamedFile> {
    NamedFile::open("app/index.html").await.ok()
}

// api calls
#[get("/call/<user>/search?<query..>")]
fn search(user: &str, query: Query, nwt_event: NwtEvent) -> String {
    format!(
        "call search triggered for {} on since={} until={} limit={} page={} tag={:?} tagLogic={:?} order={:?} folder={:?} with token {:?}",
        user, query.since, query.until, query.limit, query.page, query.tag, query.tag_logic, query.order, query.folder, nwt_event
    )
}

#[post("/call/<user>/save?<filename>", data = "<upload>")]
async fn save(
    user: &str,
    filename: Vec<&str>,
    mut upload: Form<Uploads<'_>>,
    nwt_event: NwtEvent,
) -> String {
    // todo create error list of files that couldn't be saved
    for file in &mut upload.upload {
        let (path, raw_name, content_type) = match (file.path(), file.raw_name(), file.content_type()) {
            (Some(path), Some(raw_name), Some(content_type)) => (path, raw_name, content_type),
            _ => 
            return format!(
                "couldn't find saved path, content type and/or file name {:?} {:?} {:?} for file {:?}",
                file.path(),
                file.content_type(),
                file.raw_name(),
                file
            )
        };
        let (hash, ext) = match (
            sha256_file(&path),
            Path::new(raw_name.dangerous_unsafe_unsanitized_raw().as_str())
                .extension()
                .and_then(OsStr::to_str),
        ) {
            (Ok(hash), Some(ext)) => (hash, String::from(ext)),
            _ => return format!("hash unsuccessful")
        };
        let hash_filename = [&hash, ext.as_str()].join(".");
        let expected_type = match ContentType::from_extension(ext.as_str()) {
            Some(expected_type) => expected_type,
            _ => return format!(
                "file types couldn't be verified, expected {:?} got {:?}",
                file.content_type(),
                ContentType::from_extension(ext.as_str()),
            )
        };
        if filename.iter().any(|name| {
            *name == hash_filename.as_str() && *content_type == expected_type
        }) {
            let save_path = [
                "static",
                nwt_event.username.as_str(),
                expected_type.media_type().top().as_str(),
                &hash_filename,
            ]
            .join("/");
            if let Ok(_) = file.persist_to(save_path.as_str()).await {
            } else {
                return format!("failed to save to path {:?}", save_path,);
            }
            // None
        } else {
            // Some(file)
            return format!(
                "filenamehash {} didn't match {:?} for file {:?}",
                hash,
                filename,
                file.raw_name()
            );
        }
    }

    format!(
        "call save triggered for {} with {:?} named {:?} with nwtEvent {:?}",
        user,
        upload,
        filename,
        nwt_event, 
    )
}

#[catch(default)]
fn default_catcher(status: Status, request: &Request) -> String {
    format!("error {status} for request {request}")
}

// /// calculates sha256 digest as lowercase hex string
// fn sha256_digest(path: &Path) -> io::Result<String> {
//     let mut path_buf = PathBuf::new();
//     path_buf.push(path);
//     let input = File::open(&path_buf)?;
//     let mut reader = BufReader::new(input);

//     let digest = {
//         let mut hasher = Sha256::new();
//         let mut buffer = [0; 1024];
//         loop {
//             let count = reader.read(&mut buffer)?;
//             if count == 0 {
//                 break;
//             }
//             hasher.update(&buffer[..count]);
//         }
//         hasher.finalize()
//     };
//     Ok(HEXLOWER.encode(digest.as_ref()))
// }
/// calculates sha256 digest as lowercase hex string
fn sha256_file(path: &Path) -> io::Result<String> {
    let mut path_buf = PathBuf::new();
    path_buf.push(path);
    let input = File::open(&path_buf)?;
    let mut reader = BufReader::new(input);
    sha256_digest(&mut reader)
}
fn sha256_string(s: &str) -> io::Result<String> {
    let mut reader = s.as_bytes();
    sha256_digest(&mut reader)
}
fn sha256_digest(reader: &mut dyn io::BufRead) -> io::Result<String> {
    // let mut path_buf = PathBuf::new();
    // path_buf.push(path);
    // let input = File::open(&path_buf)?;
    // let mut reader = BufReader::new(input);
    // let mut reader = s.as_bytes();

    let digest = {
        let mut hasher = Sha256::new();
        let mut buffer = [0; 1024];
        loop {
            let count = reader.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
        }
        hasher.finalize()
    };
    Ok(HEXLOWER.encode(digest.as_ref()))
}

/// Utility function used to parse hex into a target u8 buffer. Returns
/// the number of bytes converted or an error if it encounters an invalid
/// character or unexpected end of string.
fn from_hex(hex: &str, target: &mut [u8]) -> Result<usize, ()> {
    if hex.len() % 2 == 1 || hex.len() > target.len() * 2 {
        return Err(());
    }

    let mut b = 0;
    let mut idx = 0;
    for c in hex.bytes() {
        b <<= 4;
        match c {
            b'A'..=b'F' => b |= c - b'A' + 10,
            b'a'..=b'f' => b |= c - b'a' + 10,
            b'0'..=b'9' => b |= c - b'0',
            _ => return Err(()),
        }
        if (idx & 1) == 1 {
            target[idx / 2] = b;
            b = 0;
        }
        idx += 1;
    }
    Ok(idx / 2)
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount(
            "/",
            routes![
                search,
                save,
                index,
                index_resources,
                nip05,
                user_index,
                folder_index,
            ],
        )
        // .mount("/", routes![index, index_resources])
        .mount("/", FileServer::from("static"))
        .register("/", catchers![default_catcher])
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for NwtEvent {
    type Error = NwtError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // grab nwt from auth header
        fn nwt_from_header(req: &'_ Request<'_>) -> Result<String, Outcome<Status, NwtError>> {
            let auth_header = match req.headers().get_one("Authorization") {
                Some(v) => v,
                None => return Err(Outcome::Failure((Status::BadRequest, NwtError::Missing))),
            };
            if !auth_header.starts_with("Bearer ") {
                return Err(Outcome::Failure((Status::BadRequest, NwtError::Missing)));
            }
            Ok(auth_header.trim_start_matches("Bearer ").to_owned())
        }

        // decode nwt to json string
        fn decode_nwt(nwt: String) -> Result<String, Outcome<Status, NwtError>> {
            let nwt_u8 = match base64_url::decode(nwt.as_str()) {
                Ok(nwt_u8) => nwt_u8,
                Err(_) => return Err(Outcome::Failure((Status::BadRequest, NwtError::Invalid))),
            };
            match String::from_utf8(nwt_u8) {
                Ok(nwt_json) => Ok(nwt_json),
                Err(_) => return Err(Outcome::Failure((Status::BadRequest, NwtError::Invalid))),
            }
        }

        // deserialize nwt json to NwtEvent
        fn deserialize_nwt(nwt_json: String) -> Result<NwtEvent, Outcome<Status, NwtError>> {
            // let value: Value = from_str(nwt_json.as_str()).unwrap();
            // println!("json {:?}", value);
            let mut nwt_event: NwtEvent = match from_str(nwt_json.as_str()) {
                Ok(nwt_event) => nwt_event,
                Err(_) => return Err(Outcome::Failure((Status::BadRequest, NwtError::Invalid))),
            };
            // todo get actual user name
            // let username = getUsername(nwt_event.pubkey);
            let users_json = read_to_string("./static/users.json").expect("Unable to read file");
            let users: Value = from_str(users_json.as_str()).expect("JSON does not have correct format.");
            let mut username = String::new();
            for (user, pubkey) in users["names"].as_object().unwrap() {
                if pubkey.as_str().unwrap() == nwt_event.pubkey.as_str() {
                    // username = String::from(user);
                    username.push_str(user.as_str());
                }
            };
            if username.is_empty() {
                return Err(Outcome::Failure((Status::BadRequest, NwtError::Invalid)));
            };
            nwt_event.username = username;
            Ok(nwt_event)
        }

        // validate nwt event
        fn validate_nwt(nwt_event: &NwtEvent) -> Result<(), Outcome<Status, NwtError>> {
            if nwt_event.generate_id() != nwt_event.id {
                return Err(Outcome::Failure((Status::BadRequest, NwtError::Invalid)))
            };

            let mut pubkey_bytes: [u8; 32] = [0; 32];
            // from_hex(nwt_event.pubkey.as_str(), &mut pubkey_bytes);
            let mut id_bytes: [u8; 32] = [0; 32];
            // from_hex(nwt_event.id.as_str(), &mut id_bytes);
            let mut sig_bytes: [u8; 64] = [0; 64];
            // from_hex(nwt_event.sig.as_str(), &mut sig_bytes);

            if from_hex(nwt_event.pubkey.as_str(), &mut pubkey_bytes).is_err() ||
                from_hex(nwt_event.id.as_str(), &mut id_bytes).is_err() ||
                from_hex(nwt_event.sig.as_str(), &mut sig_bytes).is_err()
            {
                // (Ok(p), Ok(i), Ok(s)) => (Ok(p), Ok(i), Ok(s)),
                return Err(Outcome::Failure((Status::BadRequest, NwtError::Invalid)))
            };

            let (pubkey, message, sig) = match (
                XOnlyPublicKey::from_slice(&pubkey_bytes),
                Message::from_slice(&id_bytes),
                Signature::from_slice(&sig_bytes),
            ) {
                (Ok(pubkey), Ok(message), Ok(sig)) => (pubkey, message, sig),
                _ => return Err(Outcome::Failure((Status::BadRequest, NwtError::Invalid))),
            };
            let secp = Secp256k1::verification_only();
            match secp.verify_schnorr(&sig, &message, &pubkey) {
                Ok(()) => Ok(()),
                Err(_) => Err(Outcome::Failure((Status::BadRequest, NwtError::Invalid))),
            }
        }

        // check nwt permissions
        fn check_nwt_permissions(
            req: &'_ Request<'_>,
            nwt_event: &NwtEvent,
        ) -> Result<(), Outcome<Status, NwtError>> {
            if req.method() != nwt_event.content.method {
                return Err(Outcome::Failure((Status::BadRequest, NwtError::Invalid)));
            };

            let (path, query) = match (req.uri().path(), req.uri().query()) {
                (path, Some(query)) => (path, query.as_str()),
                _ => return Err(Outcome::Failure((Status::BadRequest, NwtError::Invalid))),
            };
            // todo get actual domain
            let req_uri = format!("http://localhost:8000{}?{}", path, query);
            if req_uri != nwt_event.content.uri {
                return Err(Outcome::Failure((Status::BadRequest, NwtError::Invalid)));
            }

            if !Path::new(path.as_str()).starts_with(format!("/call/{}", nwt_event.username).as_str()) {
                return Err(Outcome::Failure((Status::BadRequest, NwtError::Invalid)));
            };

            Ok(())
        }

        fn authorize<'r>(req: &'_ Request<'_>) -> Result<NwtEvent, Outcome<Status, NwtError>> {
            let nwt = nwt_from_header(&req)?;
            let nwt_json = decode_nwt(nwt)?;

            // deserialize json string to NwtEvent
            let nwt_event = deserialize_nwt(nwt_json)?;
            // let nwt_event: NwtEvent = match from_str(nwt_json.as_str()) {
            //     Ok(nwt_event) => nwt_event,
            //     Err(_) => return Err(Outcome::Failure((Status::BadRequest, NwtError::Invalid))),
            // };

            validate_nwt(&nwt_event)?;
            check_nwt_permissions(req, &nwt_event)?;

            Ok(nwt_event)
        }

        match authorize(&req) {
            Ok(nwt_event) => Outcome::Success(nwt_event),
            Err(Outcome::Failure(e)) => Outcome::Failure(e),
            Err(_) => Outcome::Failure((Status::BadRequest, NwtError::Invalid)),
        }
    }
}
