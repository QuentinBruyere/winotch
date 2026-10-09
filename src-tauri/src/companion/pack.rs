//! A custom companion's pack (ADR-0015): `companion.json` and a PNG sheet,
//! turned into the pictures the front draws for the built-in companions
//! (rows of characters). Pure logic: reading files is `library`'s job.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

/// The states every companion plays, and the moods of the Companion module.
const TONES: [&str; 6] = [
    "neutral",
    "active",
    "attention",
    "question",
    "success",
    "error",
];
const MOODS: [&str; 8] = [
    "dance", "run", "watch", "petted", "stretch", "idle", "sleep", "alarm",
];
/// What a state without its own animation plays.
const FALLBACKS: [(&str, &str); 4] = [
    ("attention", "active"),
    ("question", "active"),
    ("error", "active"),
    ("success", "neutral"),
];

/// A pack's file (sheet or zip) may not be bigger.
pub const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_SHEET_SIDE: u32 = 2048;
const MAX_FRAMES: u32 = 512;
const MIN_HEIGHT: u32 = 8;
const MAX_HEIGHT: u32 = 64;
/// A pixel this opaque or more is drawn.
const OPAQUE: u8 = 128;
const DEFAULT_FPS: f32 = 6.0;
const MAX_FPS: f32 = 60.0;
/// The characters standing for the colors of an `own` colors pack; `.` is
/// an empty pixel.
const COLOR_CHARS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
/// An empty pixel, and a pixel of a tinted pack.
const EMPTY: char = '.';
const FILLED: char = '#';

/// Why a pack is refused; shown to the user (`message`).
#[derive(Debug, Clone, PartialEq)]
pub enum PackError {
    MissingManifest,
    InvalidZip,
    InvalidManifest(String),
    MissingSheet(String),
    InvalidSheet,
    TooBig,
    FrameSize,
    SheetSize,
    TooManyFrames,
    TooManyColors,
    MissingAnimation(&'static str),
    UnknownAnimation(String),
    EmptyAnimation(String),
    InvalidFrame { animation: String, frame: String },
    FrameOutOfRange { animation: String, frame: u32 },
    InvalidFps(String),
}

impl PackError {
    /// The reason, in the current language.
    pub fn message(&self) -> String {
        use PackError::*;
        match self {
            MissingManifest => crate::t!("companion.pack.missing_manifest"),
            InvalidZip => crate::t!("companion.pack.invalid_zip"),
            InvalidManifest(detail) => {
                crate::t!("companion.pack.invalid_manifest", detail = detail)
            }
            MissingSheet(file) => crate::t!("companion.pack.missing_sheet", file = file),
            InvalidSheet => crate::t!("companion.pack.invalid_sheet"),
            TooBig => crate::t!("companion.pack.too_big"),
            FrameSize => crate::t!("companion.pack.frame_size"),
            SheetSize => crate::t!("companion.pack.sheet_size"),
            TooManyFrames => crate::t!("companion.pack.too_many_frames", max = MAX_FRAMES),
            TooManyColors => crate::t!("companion.pack.too_many_colors", max = COLOR_CHARS.len()),
            MissingAnimation(name) => crate::t!("companion.pack.missing_animation", name = name),
            UnknownAnimation(name) => crate::t!("companion.pack.unknown_animation", name = name),
            EmptyAnimation(name) => crate::t!("companion.pack.empty_animation", name = name),
            InvalidFrame { animation, frame } => crate::t!(
                "companion.pack.invalid_frame",
                name = animation,
                frame = frame
            ),
            FrameOutOfRange { animation, frame } => crate::t!(
                "companion.pack.frame_out_of_range",
                name = animation,
                frame = frame
            ),
            InvalidFps(name) => crate::t!("companion.pack.invalid_fps", name = name),
        }
    }
}

/// `companion.json`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub name: String,
    #[serde(default = "default_sheet")]
    pub sheet: String,
    frame_width: u32,
    frame_height: u32,
    #[serde(default)]
    colors: Colors,
    fps: Option<f32>,
    animations: BTreeMap<String, AnimationSpec>,
}

fn default_sheet() -> String {
    "sheet.png".into()
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
enum Colors {
    /// Only the shape counts: winotch paints it (state, chosen color).
    #[default]
    Tint,
    /// The pack keeps its own colors.
    Own,
}

#[derive(Debug, Deserialize)]
struct AnimationSpec {
    #[serde(default)]
    intro: Vec<FrameRef>,
    #[serde(default)]
    frames: Vec<FrameRef>,
    #[serde(default)]
    scenes: Vec<Vec<FrameRef>>,
    fps: Option<f32>,
}

/// A picture of the sheet: its number, or a range `"a-b"`.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum FrameRef {
    Index(u32),
    Text(String),
}

/// A picture: rows of characters, `.` empty.
type Frame = Vec<String>;

/// One animation, as the front's `Animation` (src/lib/companions/companion.ts).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Animation {
    fps: f32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    intro: Vec<Frame>,
    frames: Vec<Frame>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scenes: Option<Vec<Vec<Frame>>>,
}

/// A companion, as the front's `Companion`, plus the palette of an `own`
/// colors pack (character → `#rrggbb`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Definition {
    width: u32,
    height: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    palette: Option<BTreeMap<char, String>>,
    animations: BTreeMap<String, Animation>,
    moods: BTreeMap<String, Animation>,
}

impl Manifest {
    pub fn parse(json: &[u8]) -> Result<Manifest, PackError> {
        let manifest: Manifest =
            serde_json::from_slice(json).map_err(|e| PackError::InvalidManifest(e.to_string()))?;
        if manifest.name.trim().is_empty() {
            return Err(PackError::InvalidManifest("name".into()));
        }
        // Next to the manifest, never elsewhere.
        if manifest.sheet.contains(['/', '\\']) || manifest.sheet.starts_with('.') {
            return Err(PackError::InvalidManifest("sheet".into()));
        }
        Ok(manifest)
    }

    pub fn name(&self) -> String {
        self.name.trim().to_owned()
    }

    /// The pictures from `sheet`, the PNG the manifest names.
    pub fn build(&self, sheet: &[u8]) -> Result<Definition, PackError> {
        let (width, height) = (self.frame_width, self.frame_height);
        if width * 2 != height * 3 || !(MIN_HEIGHT..=MAX_HEIGHT).contains(&height) {
            return Err(PackError::FrameSize);
        }
        if sheet.len() as u64 > MAX_FILE_BYTES {
            return Err(PackError::TooBig);
        }
        let image = image::load_from_memory_with_format(sheet, image::ImageFormat::Png)
            .map_err(|_| PackError::InvalidSheet)?
            .to_rgba8();
        if image.width() > MAX_SHEET_SIDE || image.height() > MAX_SHEET_SIDE {
            return Err(PackError::TooBig);
        }
        if image.width() % width != 0 || image.height() % height != 0 || image.width() == 0 {
            return Err(PackError::SheetSize);
        }
        let columns = image.width() / width;
        let count = columns * (image.height() / height);
        if count > MAX_FRAMES {
            return Err(PackError::TooManyFrames);
        }

        let mut cutter = Cutter {
            image: &image,
            width,
            height,
            columns,
            count,
            colors: self.colors,
            palette: HashMap::new(),
            frames: HashMap::new(),
        };
        let default_fps = self.fps.unwrap_or(DEFAULT_FPS);
        let mut animations = BTreeMap::new();
        let mut moods = BTreeMap::new();
        for (name, spec) in &self.animations {
            let target = if TONES.contains(&name.as_str()) {
                &mut animations
            } else if MOODS.contains(&name.as_str()) {
                &mut moods
            } else {
                return Err(PackError::UnknownAnimation(name.clone()));
            };
            target.insert(name.clone(), cutter.animation(name, spec, default_fps)?);
        }
        for required in ["neutral", "active"] {
            if !animations.contains_key(required) {
                return Err(PackError::MissingAnimation(required));
            }
        }
        for (tone, borrowed) in FALLBACKS {
            if !animations.contains_key(tone) {
                let animation = animations[borrowed].clone();
                animations.insert(tone.to_owned(), animation);
            }
        }

        let palette = (self.colors == Colors::Own).then(|| {
            cutter
                .palette
                .iter()
                .map(|([r, g, b], c)| (*c, format!("#{r:02x}{g:02x}{b:02x}")))
                .collect()
        });
        Ok(Definition {
            width,
            height,
            palette,
            animations,
            moods,
        })
    }
}

/// Cuts the sheet's pictures, each once, and names their colors.
struct Cutter<'a> {
    image: &'a image::RgbaImage,
    width: u32,
    height: u32,
    columns: u32,
    count: u32,
    colors: Colors,
    palette: HashMap<[u8; 3], char>,
    frames: HashMap<u32, Frame>,
}

impl Cutter<'_> {
    fn animation(
        &mut self,
        name: &str,
        spec: &AnimationSpec,
        default_fps: f32,
    ) -> Result<Animation, PackError> {
        let fps = spec.fps.unwrap_or(default_fps);
        if !(fps > 0.0 && fps <= MAX_FPS) {
            return Err(PackError::InvalidFps(name.to_owned()));
        }
        let frames = self.frames(name, &spec.frames)?;
        if frames.is_empty() {
            return Err(PackError::EmptyAnimation(name.to_owned()));
        }
        let intro = self.frames(name, &spec.intro)?;
        let mut scenes = Vec::new();
        for scene in &spec.scenes {
            let scene = self.frames(name, scene)?;
            if !scene.is_empty() {
                scenes.push(scene);
            }
        }
        Ok(Animation {
            fps,
            intro,
            frames,
            scenes: (!scenes.is_empty()).then_some(scenes),
        })
    }

    fn frames(&mut self, name: &str, refs: &[FrameRef]) -> Result<Vec<Frame>, PackError> {
        let mut frames = Vec::new();
        for index in indices(name, refs)? {
            if index >= self.count {
                return Err(PackError::FrameOutOfRange {
                    animation: name.to_owned(),
                    frame: index,
                });
            }
            frames.push(self.frame(index)?);
        }
        Ok(frames)
    }

    fn frame(&mut self, index: u32) -> Result<Frame, PackError> {
        if let Some(frame) = self.frames.get(&index) {
            return Ok(frame.clone());
        }
        let left = (index % self.columns) * self.width;
        let top = (index / self.columns) * self.height;
        let mut frame = Vec::with_capacity(self.height as usize);
        for y in top..top + self.height {
            let mut row = String::with_capacity(self.width as usize);
            for x in left..left + self.width {
                let [r, g, b, a] = self.image.get_pixel(x, y).0;
                row.push(if a < OPAQUE {
                    EMPTY
                } else if self.colors == Colors::Tint {
                    FILLED
                } else {
                    self.color([r, g, b])?
                });
            }
            frame.push(row);
        }
        self.frames.insert(index, frame.clone());
        Ok(frame)
    }

    fn color(&mut self, rgb: [u8; 3]) -> Result<char, PackError> {
        if let Some(c) = self.palette.get(&rgb) {
            return Ok(*c);
        }
        let c = COLOR_CHARS
            .chars()
            .nth(self.palette.len())
            .ok_or(PackError::TooManyColors)?;
        self.palette.insert(rgb, c);
        Ok(c)
    }
}

/// The picture numbers, ranges spelled out.
fn indices(name: &str, refs: &[FrameRef]) -> Result<Vec<u32>, PackError> {
    let mut out = Vec::new();
    for r in refs {
        match r {
            FrameRef::Index(i) => out.push(*i),
            FrameRef::Text(text) => {
                let invalid = || PackError::InvalidFrame {
                    animation: name.to_owned(),
                    frame: text.clone(),
                };
                let (start, end) = match text.split_once('-') {
                    Some((a, b)) => (a.trim().parse::<u32>(), b.trim().parse::<u32>()),
                    None => (text.trim().parse::<u32>(), text.trim().parse::<u32>()),
                };
                let (Ok(start), Ok(end)) = (start, end) else {
                    return Err(invalid());
                };
                if start > end || end - start >= MAX_FRAMES {
                    return Err(invalid());
                }
                out.extend(start..=end);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A PNG of `columns` × `rows` pictures of 12 × 8 pixels; `paint` gives
    /// each pixel's color (alpha included).
    fn sheet(columns: u32, rows: u32, paint: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
        let image =
            image::RgbaImage::from_fn(columns * 12, rows * 8, |x, y| image::Rgba(paint(x, y)));
        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgba8(image)
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .unwrap();
        bytes
    }

    fn manifest(json: &str) -> Manifest {
        Manifest::parse(json.as_bytes()).unwrap()
    }

    const MINIMAL: &str = r#"{
        "name": " Pixel ",
        "frameWidth": 12, "frameHeight": 8,
        "animations": {
            "neutral": { "frames": [0] },
            "active": { "intro": [1], "frames": ["2-3"], "fps": 8 }
        }
    }"#;

    #[test]
    fn a_tinted_pack_becomes_pictures_of_hashes() {
        // Picture n has its column n opaque.
        let png = sheet(4, 1, |x, _| {
            [9, 9, 9, if x % 12 == x / 12 { 255 } else { 0 }]
        });
        let manifest = manifest(MINIMAL);
        assert_eq!(manifest.name(), "Pixel");
        let definition = manifest.build(&png).unwrap();
        assert_eq!(definition.palette, None);
        let neutral = &definition.animations["neutral"];
        assert_eq!(neutral.fps, DEFAULT_FPS);
        assert_eq!(neutral.frames[0][0], format!("#{}", ".".repeat(11)));
        let active = &definition.animations["active"];
        assert_eq!(active.fps, 8.0);
        assert_eq!(active.intro.len(), 1);
        assert_eq!(active.frames.len(), 2);
        assert_eq!(active.frames[1][7], format!("...#{}", ".".repeat(8)));
    }

    #[test]
    fn missing_states_borrow_working_or_rest() {
        let png = sheet(4, 1, |_, _| [0, 0, 0, 255]);
        let definition = manifest(MINIMAL).build(&png).unwrap();
        assert_eq!(definition.animations.len(), TONES.len());
        assert_eq!(
            definition.animations["error"],
            definition.animations["active"]
        );
        assert_eq!(
            definition.animations["success"],
            definition.animations["neutral"]
        );
        assert!(definition.moods.is_empty());
    }

    #[test]
    fn an_own_colors_pack_keeps_a_palette() {
        let png = sheet(4, 1, |x, _| match x % 3 {
            0 => [255, 0, 0, 255],
            1 => [0, 0, 255, 200],
            _ => [0, 0, 0, 10],
        });
        let json = MINIMAL.replace("\"name\"", "\"colors\": \"own\", \"name\"");
        let definition = manifest(&json).build(&png).unwrap();
        let palette = definition.palette.unwrap();
        assert_eq!(palette[&'A'], "#ff0000");
        assert_eq!(palette[&'B'], "#0000ff");
        assert_eq!(&definition.animations["neutral"].frames[0][0][..3], "AB.");
    }

    #[test]
    fn mistakes_are_named() {
        let png = sheet(4, 1, |_, _| [0, 0, 0, 255]);
        let build = |json: &str| Manifest::parse(json.as_bytes()).and_then(|m| m.build(&png));
        assert_eq!(
            build(&MINIMAL.replace("\"neutral\"", "\"napping\"")),
            Err(PackError::UnknownAnimation("napping".into()))
        );
        assert_eq!(
            build(&MINIMAL.replace("\"neutral\": { \"frames\": [0] },", "")),
            Err(PackError::MissingAnimation("neutral"))
        );
        assert_eq!(
            build(&MINIMAL.replace("\"2-3\"", "\"2-4\"")),
            Err(PackError::FrameOutOfRange {
                animation: "active".into(),
                frame: 4
            })
        );
        assert_eq!(
            build(&MINIMAL.replace("\"2-3\"", "\"3-2\"")),
            Err(PackError::InvalidFrame {
                animation: "active".into(),
                frame: "3-2".into()
            })
        );
        assert_eq!(
            build(&MINIMAL.replace("\"frameWidth\": 12", "\"frameWidth\": 10")),
            Err(PackError::FrameSize)
        );
        assert_eq!(
            build(&MINIMAL.replace("\"fps\": 8", "\"fps\": 0")),
            Err(PackError::InvalidFps("active".into()))
        );
        assert!(matches!(
            build(&MINIMAL.replace("\"name\": \" Pixel \",", "")),
            Err(PackError::InvalidManifest(_))
        ));
        assert_eq!(
            build(&MINIMAL.replace("\"name\"", "\"sheet\": \"../x.png\", \"name\"")),
            Err(PackError::InvalidManifest("sheet".into()))
        );
    }

    #[test]
    fn a_sheet_must_hold_whole_pictures() {
        let mut image = image::RgbaImage::new(30, 8);
        image.put_pixel(0, 0, image::Rgba([0, 0, 0, 255]));
        let mut png = Vec::new();
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        assert_eq!(manifest(MINIMAL).build(&png), Err(PackError::SheetSize));
        assert_eq!(
            manifest(MINIMAL).build(b"not a png"),
            Err(PackError::InvalidSheet)
        );
    }

    #[test]
    fn every_error_has_a_message() {
        let errors = [
            PackError::MissingManifest,
            PackError::InvalidZip,
            PackError::InvalidManifest("x".into()),
            PackError::MissingSheet("x".into()),
            PackError::InvalidSheet,
            PackError::TooBig,
            PackError::FrameSize,
            PackError::SheetSize,
            PackError::TooManyFrames,
            PackError::TooManyColors,
            PackError::MissingAnimation("x"),
            PackError::UnknownAnimation("x".into()),
            PackError::EmptyAnimation("x".into()),
            PackError::InvalidFrame {
                animation: "x".into(),
                frame: "y".into(),
            },
            PackError::FrameOutOfRange {
                animation: "x".into(),
                frame: 1,
            },
            PackError::InvalidFps("x".into()),
        ];
        for error in errors {
            assert!(!error.message().starts_with("companion.pack."), "{error:?}");
        }
    }
}
