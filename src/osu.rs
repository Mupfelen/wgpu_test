use std::ascii::Char::LessThanSign;
use std::collections::{BTreeMap, HashSet};
use std::error::Error;
use std::collections::HashMap;
use egui::Shape::LineSegment;
use egui::TextBuffer;

pub enum OsuObject {
    Circle(OsuCircle),
    Slider(OsuSlider),
    Spinner(OsuSpinner),
}

impl OsuObject {
    pub fn time(&self) -> u32 {
        match self {
            OsuObject::Circle(circle) => circle.time,
            OsuObject::Slider(slider) => slider.time,
            OsuObject::Spinner(spinner) => spinner.time,
        }
    }
}

pub struct OsuCircle {
    pub x: f32,
    pub y: f32,
    pub time: u32,
}

pub struct OsuSpinner {
    pub time: u32,
    pub end_time: u32,
}

pub struct OsuSlider {
    pub x: f32,
    pub y: f32,
    pub time: u32,
    //pub end_time: u32,
    pub curve_type: String,
    pub curve_points: Vec<(f32, f32)>,
    pub repeat: u32,
    pub pixel_length: f32,
}

pub struct OsuMap {
    pub objects: BTreeMap<u64, OsuObject>,
    pub name: String,
    pub artist: String,
    pub creator: String,
}

impl OsuMap {
    pub fn new() -> OsuMap {
        OsuMap {
            objects: BTreeMap::new(),
            name: String::new(),
            artist: String::new(),
            creator: String::new(),
        }
    }

    pub fn from_file(file: &str) -> Result<OsuMap, Box<dyn Error>> {
        let sections = HashSet::from(["General", "Editor", "Metadata", "Difficulty", "Events", "TimingPoints", "Colours", "HitObjects"]);
        let file = std::fs::read_to_string(file)?;
        let mut map = OsuMap::new();
        let mut mode = "";
        for line in file.lines() {
            if sections.contains(line) {
                mode = line;
                continue;
            }

            match mode {
                "HitObjects" => {
                    let object = Self::parse_hit_object(line);
                    map.objects.insert(object.time() as u64, object);
                },
                "TimingPoints" => {}
                _ => {}
            }
        }

        Ok(());
    }

    fn parse_hit_object(line: &str) -> OsuObject {
        let mut properties = line.split(",").into_iter();
        let x = properties.next().unwrap().parse::<f32>().unwrap();
        let y = properties.next().unwrap().parse::<f32>().unwrap();
        let time = properties.next().unwrap().parse::<u32>().unwrap();
        let object_type = properties.next().unwrap().parse::<u32>().unwrap();
        match object_type {
            0 => OsuObject::Circle(OsuCircle { x, y, time }),
            1 => {
                properties.next().unwrap();
                let curve_type = properties.next().unwrap();
                let curve_points = properties.next().unwrap().split("|").into_iter().map(|point| {
                    let mut point = point.split(":");
                    let x = point.next().unwrap().parse::<f32>().unwrap();
                    let y = point.next().unwrap().parse::<f32>().unwrap();
                    (x, y)
                }).collect();
                let repeat = properties.next().unwrap().parse::<u32>().unwrap();
                let pixel_length = properties.next().unwrap().parse::<f32>().unwrap();

                OsuObject::Slider(OsuSlider {
                    x,
                    y,
                    time,
                    curve_type: curve_type.to_string(),
                    curve_points,
                    repeat,
                    pixel_length,
                })
            }
            3 => {
                properties.next().unwrap();
                let end_time = properties.next().unwrap().parse::<u32>().unwrap();
                OsuObject::Spinner(OsuSpinner { time, end_time })
            }
            _ => panic!("Invalid object type"),
        }
    }
    
    fn parse_timing_point(line: &str) {}
}
