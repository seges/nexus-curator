extern crate reqwest;
extern crate serde;
extern crate semver_rs;

use serde::{Deserialize};

use reqwest::header;

use std::collections::HashMap;
use semver_rs::{Version, Range, Options, Error, compare};
use std::cmp::Ordering;
use std::env;


#[derive(Deserialize)]
struct Asset {
    downloadUrl: String,
    path: String,
    id: String
}

#[derive(Deserialize)]
struct Component {
    id: String,
    repository: String,
    format: String,
    group: Option<String>,
    name: String,
    version: String,
    assets: Vec<Asset>
}

#[derive(Deserialize)]
struct List {
    items: Vec<Component>,
    continuationToken: Option<String>
}

fn extract_semver(component: &Component) -> Result<Version, Error> {
    let opts = Options::builder().loose(true).include_prerelease(true).build();

    let split: Vec<&str> = component.version.split(".").collect();
    // println!("version segments {}",split.len());

    if split.len() == 4 {
        let first_three = format!("{}.{}.{}-{}", split[0], split[1], split[2], split[3]);
        Version::new(&first_three).with_options(opts.clone()).parse()
    } else {
        Version::new(&component.version).with_options(opts.clone()).parse()
    }
}

fn main() -> Result<(), Box<std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 7 {
        println!("nexus-curator <repo url> <user base64> <pass base64> <nexus repository name> <fullImageName> <less-than sem-version>");
        println!("");
        println!("To get base64 encoded strings use: `echo -n 'admin' | openssl base64`");
        std::process::exit(42);
    }

    let client = reqwest::Client::new();

    let mut headers = header::HeaderMap::new();
    // headers.insert(header::AUTHORIZATION, header::HeaderValue::from_static("secret"));

    // get a client builder
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .cookie_store(true)
        .build()?;


    let mut params = HashMap::new();
    params.insert("username", &args[2]);
    params.insert("password", &args[3]);
    
    let repo_url = &args[1];
    let login_url = format!("{}/service/rapture/session", repo_url);
    let res = client.post(&login_url).form(&params)
    .send()?;

    println!("{:#?}", res);

    let repository = &args[4];
    let image_name = &args[5];
    let search_url = format!("{}/service/rest/v1/search?repository={}&docker.imageName={}", repo_url, repository, image_name);
    println!("searching {}", &search_url);
    let mut res = client.get(&search_url).send()?;

    match res.json() {
        Err(e) => {
            if e.is_serialization() {
                // let serde_error = match e.get_ref() {
                //     None => return,
                //     Some(err) => err,
                // };
                // println!("problem parsing information {}", serde_error);
                println!("err {}", e);
            }
        },
        Ok(json) => {
            let list: List = json;
            println!("search result count {}", list.items.len());

            for component in list.items.iter() {
                println!("component {} in version {:#?}", component.name, component.version);

                let semver = extract_semver(component);
                
                let fixver = Version::new(&args[6]).parse();
                // println!("fixver {}", fixver?);

                // println!("component is less then baseline {}", semver? < fixver?);

                if semver? < fixver? {
                    let url = format!("{}/service/rest/v1/components/{}", repo_url, component.id);
                    println!("deleting name {}; version {}; url {}", component.name, component.version, url);
                    let del = client.delete(&url).send()?;
                    println!("delete response {:#?}", del);
                }
            }
        }
    }

    // println!("{:#?}", list.continuation_token);
    Ok(())
}
