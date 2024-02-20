use egui::Shape::LineSegment;
use egui::TextBuffer;
use std::collections::HashMap;
use std::collections::{BTreeMap, HashSet};
use std::error::Error;

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

pub enum TimingPoint {
    Inherited(InheritedTimingPoint),
    Uninherited(UninheritedTimingPoint),
}

pub struct UninheritedTimingPoint {
    pub time: u32,
    pub bpm: f32,
    pub meter: u32,
    pub sample_set: u32,
    pub sample_index: u32,
    pub volume: u32,
    pub effects: u32
}

pub struct InheritedTimingPoint {
    pub time: u32,
    pub slider_multiplier: f32,
    pub sample_set: u32,
    pub sample_index: u32,
    pub volume: u32,
    pub effects: u32
}

pub struct OsuMap {
    pub objects: BTreeMap<u64, OsuObject>,
    pub timing_points: BTreeMap<u64, OsuObject>,
    pub name: String,
    pub artist: String,
    pub creator: String,
}

impl OsuMap {
    pub fn new() -> OsuMap {
        OsuMap {
            objects: BTreeMap::new(),
            timing_points: BTreeMap::new(),
            name: String::new(),
            artist: String::new(),
            creator: String::new(),
        }
    }

    pub fn from_file(file: &str) -> Result<OsuMap, Box<dyn Error>> {
        let sections = HashSet::from([
            "General",
            "Editor",
            "Metadata",
            "Difficulty",
            "Events",
            "TimingPoints",
            "Colours",
            "HitObjects",
        ]);
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
                }
                "TimingPoints" => {}
                _ => {}
            }
        }

        Err("Invalid file".into())
    }

    fn parse_hit_object(line: &str) -> Option<OsuObject> {
        let mut properties = line.split(",").into_iter();
        let x = properties.next()?.parse::<f32>()?;
        let y = properties.next()?.parse::<f32>()?;
        let time = properties.next()?.parse::<u32>()?;
        let object_type = properties.next()?.parse::<u32>()? & 0b1011;

        match object_type {
            1 => Some(OsuObject::Circle(OsuCircle { x, y, time })),
            2 => {
                properties.next().unwrap();
                let curve_data = properties.next().unwrap().split("|").collect::<Vec<&str>>();
                let curve_type = curve_data[0];

                let curve_points = curve_data[1..]
                    .into_iter()
                    .map(|point| {
                        let mut point = point.split(":");
                        let x = point.next().unwrap().parse::<f32>().unwrap();
                        let y = point.next().unwrap().parse::<f32>().unwrap();
                        (x, y)
                    })
                    .collect();
                let repeat = properties.next().unwrap().parse::<u32>().unwrap();
                let pixel_length = properties.next().unwrap().parse::<f32>().unwrap();

                Some(OsuObject::Slider(OsuSlider {
                    x,
                    y,
                    time,
                    curve_type: curve_type.to_string(),
                    curve_points,
                    repeat,
                    pixel_length,
                }))
            }
            8 => {
                properties.next().unwrap();
                let end_time = properties.next().unwrap().parse::<u32>().unwrap();
                Some(OsuObject::Spinner(OsuSpinner { time, end_time }))
            }
            _ => None,
        }
    }

    //TODO
    fn parse_timing_point(line: &str) -> Option<TimingPoint> {
        let mut properties = line.split(",").into_iter();
        let time = properties.next()?.parse::<u32>().unwrap();
        let bpm = properties.next()?.parse::<f32>().unwrap();
        let meter = properties.next()?.parse::<u32>().unwrap();
        let sample_set = properties.next()?.parse::<u32>().unwrap();
        let sample_index = properties.next()?.parse::<u32>().unwrap();
        let volume = properties.next()?.parse::<u32>().unwrap();
        let effects = properties.next()?.parse::<u32>().unwrap();

        if bpm > 0.0 {
            Some(TimingPoint::Uninherited(UninheritedTimingPoint {
                time,
                bpm,
                meter,
                sample_set,
                sample_index,
                volume,
                effects,
            }))
        } else {
            Some(TimingPoint::Inherited(InheritedTimingPoint {
                time,
                slider_multiplier: 100.0 / -bpm,
                sample_set,
                sample_index,
                volume,
                effects,
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hit_object() {
        let line = "339,109,757,6,0,P|361:169|338:224,1,105,0|0,0:0|0:0,0:0:0:0:";
        let object = OsuMap::parse_hit_object(line);
        match object {
            OsuObject::Slider(slider) => {
                assert_eq!(slider.x, 339.0);
                assert_eq!(slider.y, 109.0);
                assert_eq!(slider.time, 757);
                assert_eq!(slider.curve_type, "P");
                assert_eq!(slider.curve_points, vec![(361.0, 169.0), (338.0, 224.0)]);
                assert_eq!(slider.repeat, 1);
                //assert_eq!(slider.pixel_length, 0.0);
            }
            _ => panic!("Invalid object type"),
        }
    }
}
