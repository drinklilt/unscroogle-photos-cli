/*
1. Recursively scan directory for json files
2. For each json file, make sure it has a corresponding image file
3. If it does, read the .json file and extract the metadata
4. Write the exif metadata to the image file
 */

use bindet;
use bindet::types::FileType;
use img_parts::jpeg::{markers, Jpeg, JpegSegment};
use serde_json::json;
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, Read};

struct Filepair {
    json: String,
    image: String,
}

// TODO: Make this threaded
fn list_files(path: &std::path::Path, list: &mut Vec<Filepair>) -> Result<(), std::io::Error> {
    let entries = fs::read_dir(path).unwrap();
    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.is_file() {
            match path.extension() {
                Some(ext) => {
                    // For each json file, make sure it has a corresponding image file
                    if ext == "json" {
                        match fs::read_to_string(path.clone()) {
                            Ok(json) => {
                                //let v: serde_json::Value = serde_json::from_str(&json).unwrap();
                                let v: serde_json::Value = json!(json);
                                // Find image file corresponding to the json file by parsing the json and extracting the "title" field
                                let title = v["title"].as_str();
                                match title {
                                    Some(title) => {
                                        let image_path = path.with_file_name(title);
                                        if image_path.exists() {
                                            list.push(Filepair {
                                                json: path.to_str().unwrap().to_string(),
                                                image: image_path.to_str().unwrap().to_string(),
                                            });
                                        }
                                    }
                                    None => println!(
                                        "Not a valid json file: {}",
                                        path.to_str().unwrap()
                                    ),
                                }
                            }
                            Err(e) => eprintln!("Error: {}", e),
                        }
                    }
                    // the file is not a json file, so ignore it
                }
                // the file is not a json file, so ignore it
                None => (),
            }
        } else if path.is_dir() {
            list_files(&path, list)?;
        }
    }

    Ok(())
}

fn get_file_type(image_file: &File) -> Result<FileType, String> {
    // Use file reference to put the buffer into bindet
    let detect = bindet::detect(BufReader::new(image_file));
    match detect {
        Ok(detection) => match detection {
            Some(d) => match d.likely_to_be.first() {
                Some(t) => Ok(t.clone()),
                None => Err("Error: Could not detect file type".to_string()),
            },
            None => Err("Error: Could not detect file type".to_string()),
        },
        Err(_) => return Err("Error: Could not detect file type".to_string()),
    }
}

// Function to add exif metadata to an image file
fn add_exif_metadata(image_path: &str, json_path: &str) -> Result<(), String> {
    println!("{}", image_path);
    // Open the image file using OpenOptions
    let mut image_file = match OpenOptions::new().read(true).write(true).open(image_path) {
        Ok(file) => file,
        Err(e) => {
            return Err(format!(
                "Cannot open file: {}\n{}",
                image_path.clone().to_string(),
                e
            ))
        }
    };

    let file_type = match get_file_type(&image_file) {
        Ok(file_type) => file_type,
        Err(e) => return Err(e),
    };

    match file_type {
        FileType::Jpg => {
            // Parse the JSON first :)
            // let v: serde_json::Value = serde_json::from_str(&json).unwrap();
            let json_data = match fs::read_to_string(json_path) {
                Ok(json) => json!(json),
                Err(e) => return Err(format!("Could not read file!\nError: {}", e)),
            };

            // Read the metadata in the json file
            // https://www.awaresystems.be/imaging/tiff/tifftags/privateifd/exif.html
            // https://www.media.mit.edu/pia/Research/deepview/exif.html

            // DateTimeOriginal
            // The timestamp is in unix seconds (string), so like, we gotta change it into an i32
            let photo_taken_time = match json_data["photoTakenTime"]["timestamp"].as_str() {
                Some(creation_time) => match creation_time.parse::<i32>() {
                    Ok(creation_time) => creation_time,
                    Err(e) => return Err(format!("Could not parse creationTime!\nError: {}", e)),
                },
                None => return Err("Could not find creationTime in json file".to_string()),
            };
            // DateTimeDigitized
            let creation_time = match json_data["creationTime"]["timestamp"].as_str() {
                Some(creation_time) => match creation_time.parse::<i32>() {
                    Ok(creation_time) => creation_time,
                    Err(e) => return Err(format!("Could not parse creationTime!\nError: {}", e)),
                },
                None => return Err("Could not find creationTime in json file".to_string()),
            };
            // The format for DateTimeOriginal/Digitized is YYYY:MM:DD HH:MM:SS
            // Probably UTC? But exif as a whole is a mess, so who knows
        

            // Open
            let mut image_bytes = Vec::new();
            let _ = match image_file.read_to_end(&mut image_bytes) {
                Ok(_) => (),
                Err(e) => return Err(format!("Could not read file!\nError: {}", e)),
            };

            let mut jpeg = Jpeg::from_bytes(image_bytes.into());

            println!("JPG");
        }
        FileType::Png => {
            println!("PNG");
        }
        _ => println!("Not a PNG ot JPG"),
    }

    Ok(())
}

fn main() {
    // TODO: Add a command line argument to specify the directory to scan
    let path = std::path::Path::new(".");
    let mut files: Vec<Filepair> = Vec::new();
    let then = std::time::Instant::now();
    match list_files(path, &mut files) {
        Ok(_) => {
            println!(
                "Found {} files in {} ms",
                files.len(),
                then.elapsed().as_millis()
            );
            /*for file in files.iter() {
                // Now we have the path to the json file and the image file
                println!("{} {}", file.json, file.image);
            }*/
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}
