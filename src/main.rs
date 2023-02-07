use std::{env, fs::File, io, process::exit};

fn download_album(arg: &str) -> i32 {
    match arg.parse::<u64>() {
        Ok(num) => match Album::from_uid(num) {
            Ok(x) => {
                x.download();
                0
            }
            Err(x) => {
                println!("Error: {x}");
                1
            }
        },
        Err(x) => {
            println!("Error: {x}");
            1
        }
    }
}

fn download_track(arg: &str) -> i32 {
    match arg.parse::<u64>() {
        Ok(num) => match Track::from_uid(num) {
            Ok(x) => {
                x.download();
                0
            }
            Err(x) => {
                println!("Error: {x}");
                1
            }
        },
        Err(x) => {
            println!("Error: {x}");
            1
        }
    }
}

fn help() {
    println!("NeteaseCloudGetter - Download music in ease");
    println!(
        r#"    -h Show help
    -d Download tracks
    -a Download an album from a given uid
    -t Download a single track from a given uid"#
    );
}

fn main() {
    let mut args = Vec::new();

    for arg in env::args().enumerate() {
        if arg.0 == 0 && arg.1.contains("target/") {
            continue;
        }
        args.push(arg.1);
    }

    for arg in args.iter().enumerate() {
        let mut errored = false;
        let arg_str = &arg.1;
        if arg_str.contains("-") && !arg_str.contains("--") {
            let mut pure_arg = arg_str.replace("-", "");

            if pure_arg.eq("h") {
                help();
                exit(0)
            }

            if pure_arg.contains("d") && args.len() >= arg.0 {
                pure_arg = pure_arg.replace("d", "");
                if pure_arg.contains("a") {
                    pure_arg = pure_arg.replace("a", "");
                    if download_album(args.get(arg.0 + 1).unwrap()) == 1 {
                        errored = true
                    }
                }

                if pure_arg.contains("t") {
                    pure_arg = pure_arg.replace("t", "");
                    if download_track(args.get(arg.0 + 1).unwrap()) == 1 {
                        errored = true
                    }
                }
            }

            if !pure_arg.eq("") {
                println!("Unknown arguments: {pure_arg}");
                exit(1)
            }

            exit(if errored { 0 } else { 1 })
        }
    }

    help();
}

trait TrackAccess {
    fn ls_tracks(&self) -> Vec<&Track>;

    fn download(&self) {
        let mut i = 0;
        let tracks = self.ls_tracks();
        let len = tracks.len();

        println!("Tracks ({len}): \n");

        for music in &tracks {
            println!("- {} by {} in {}", music.name, music.artist, music.album);
        }

        println!("\nProceed with download? [Y/n] ");

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line.");

        if input.eq_ignore_ascii_case("Y") {
            for music in &tracks {
                i += 1;
                match File::create(format!("{}.mp3", &music.name)) {
                    Ok(file) => {
                        download_file(file, &music.get_url());
                        println!("({i}/{len}) Downloaded track {}", music.name)
                    }
                    Err(_) => {
                        let name = format!("{i}_of_{len}_errored_{}.mp3", music.id);
                        download_file(File::create(&name).unwrap(), &music.get_url());
                        println!("({i}/{len}) Downloaded track {} as {name}", music.name)
                    }
                }
            }
        }
    }
}

fn download_file(mut file: File, url: &str) {
    let resp = reqwest::blocking::get(url)
        .expect(&format!("Request failed when getting file {:?}!", file));
    let body = resp
        .bytes()
        .expect(&format!("Body invalid when getting file {:?}!", file));

    io::copy(&mut &body.to_vec()[..], &mut file).expect(&format!(
        "Failed to copy content when getting file {:?}!",
        file
    ));
}

struct Track {
    id: u64,
    name: String,
    artist: String,
    album: String,
}

impl Track {
    pub fn get_url(&self) -> String {
        format!(
            "http://music.163.com/song/media/outer/url?id={}.mp3",
            self.id
        )
    }

    pub fn from_uid(uid: u64) -> Result<Track, String> {
        let mut cycle = 0;

        while cycle < 50 {
            cycle += 0;
            let resp = reqwest::blocking::get(format!(
                "http://music.163.com/api/song/detail/?id={uid}&ids=%5B{uid}%5D"
            ))
            .expect(&format!("Request failed"));
            let body = resp.bytes().expect(&format!("Body invalid"));

            match Self::from_json(
                match &serde_json::from_str(&String::from_utf8(body.to_vec()).unwrap()) {
                    Ok(j) => j,
                    Err(_) => continue,
                },
            ) {
                Ok(track) => return Ok(track),
                Err(_) => continue,
            };
        }

        Err(format!("Can't fetch the track info of {uid}"))
    }

    fn from_json(json: &serde_json::Value) -> Result<Track, String> {
        let this = match match json["songs"].as_array() {
            Some(x) => x,
            None => return Err("Error when parsing json".to_string()),
        }[0]
        .as_object()
        {
            Some(x) => x,
            None => return Err("Error when parsing json".to_string()),
        };
        Ok(Track {
            id: match this["id"].as_u64() {
                Some(x) => x,
                None => return Err("Error when parsing id from json".to_string()),
            },
            name: match this["name"].as_str() {
                Some(x) => x.to_string(),
                None => return Err("Error when parsing name from json".to_string()),
            },
            artist: this["artists"].as_array().unwrap()[0].as_object().unwrap()["name"]
                .as_str()
                .unwrap()
                .to_string(),
            album: this["album"].as_object().unwrap()["name"]
                .as_str()
                .unwrap()
                .to_string(),
        })
    }
}

impl TrackAccess for Track {
    fn ls_tracks(&self) -> Vec<&Track> {
        let mut vec = Vec::new();
        vec.push(self);
        vec
    }
}

struct Album {
    tracks: Vec<Track>,
}

impl Album {
    pub fn from_uid(uid: u64) -> Result<Album, String> {
        let mut cycle = 0;

        while cycle < 50 {
            cycle += 1;

            let resp = reqwest::blocking::get(format!("http://music.163.com/api/album/{uid}"))
                .expect(&format!("Request failed"));
            let body = resp.bytes().expect(&format!("Body invalid"));

            match Self::from_json(
                match &serde_json::from_str(&String::from_utf8(body.to_vec()).unwrap()) {
                    Ok(j) => j,
                    Err(_) => continue,
                },
            ) {
                Ok(album) => return Ok(album),
                Err(_) => continue,
            };
        }

        Err(format!("Can't fetch the album info of {uid}"))
    }

    fn from_json(json: &serde_json::Value) -> Result<Album, String> {
        let mut tracks = Vec::new();
        let album_json = match json["album"].as_object() {
            Some(x) => x,
            None => return Result::Err("Error when parsing json".to_string()),
        };
        for t in album_json["songs"].as_array().unwrap() {
            if let Ok(x) = Track::from_uid(t.as_object().unwrap()["id"].as_u64().unwrap()) {
                tracks.push(x)
            } else {
                continue;
            }
        }
        Result::Ok(Album { tracks })
    }
}

impl TrackAccess for Album {
    fn ls_tracks(&self) -> Vec<&Track> {
        let mut tracks = Vec::new();
        for track in &self.tracks {
            tracks.push(track);
        }
        tracks
    }
}
