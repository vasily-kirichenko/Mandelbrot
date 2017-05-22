#![allow(dead_code)]

extern crate num;
extern crate image;
extern crate crossbeam;

use num::Complex;
use std::str::FromStr;
use image::ColorType;
use image::png::PNGEncoder;
use std::fs::File;
use std::io::Write;
use std::num::ParseFloatError;

fn escapes(c: Complex<f64>, limit: u32) -> Option<u32> {
    let mut z = Complex { re: 0.0, im: 0.0 };
    for i in 0..limit {
        z = z * z + c;
        if z.norm_sqr() > 4.0 {
            return Some(i);
        }
    }
    None
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ParsePairError<T> {
    ParseElementError(T),
    NoDelimiter
}

impl<T> From<T> for ParsePairError<T>  {
    fn from(x: T) -> Self {
        ParsePairError::ParseElementError(x)
    }
}

fn parse_pair<T: FromStr>(s: &str, separator: char) -> Result<(T, T), ParsePairError<T::Err>> {
    match s.find(separator) {
        None => Err(ParsePairError::NoDelimiter),
        Some(index) => {
            let e1 = T::from_str(&s[..index])?;
            let e2 = T::from_str(&s[index + 1..])?;
            Ok((e1, e2))
        }
    }
}

#[test]
fn test_parse_pair() {
    assert_eq!(parse_pair::<i32>("", ','), None);
    assert_eq!(parse_pair::<i32>("10", ','), None);
    assert_eq!(parse_pair::<i32>(",10", ','), None);
    assert_eq!(parse_pair::<i32>("10,20", ','), Some((10, 20)));
    assert_eq!(parse_pair::<i32>("10,20xy", ','), None);
    assert_eq!(parse_pair::<f64>("0.5x", ','), None);
    assert_eq!(parse_pair::<f32>("0.5x1.5", 'x'), Some((0.5, 1.5)));
}

struct Point {
    x: f64,
    y: f64
}


impl FromStr for Point {
    type Err = ParsePairError<ParseFloatError>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (x, y) = parse_pair::<f64>(s, ',')?;
        Ok(Point { x: x, y: y })
    }
}

fn pixel_to_point((lower_bound, upper_bound): (usize, usize),
                  (p0, p1): (usize, usize),
                  upper_left: &Point,
                  lower_right: &Point) -> Point
{
    let (width, height) = (lower_right.x - upper_left.x,
                           upper_left.y - lower_right.y);
    Point {
        x: upper_left.x + p0 as f64 * width / lower_bound as f64,
        y: upper_left.y - p1 as f64 * height / upper_bound as f64
    }
}

#[test]
fn test_pixel_to_point() {
    assert_eq!(
    pixel_to_point((100, 100), (25, 75), (-1.0, 1.0), (1.0, -1.0)),
    (-0.5, -0.5));
}

fn render(pixels: &mut [u8], bounds: (usize, usize), upper_left: &Point, lower_right: &Point) {
    assert!(pixels.len() == bounds.0 * bounds.1);

    for r in 0..bounds.1 {
        for c in 0..bounds.0 {
            let point = pixel_to_point(bounds, (c, r), upper_left, lower_right);
            pixels[r * bounds.0 + c] =
                match escapes(Complex { re: point.x, im: point.y }, 255) {
                    None => 0,
                    Some(count) => 255 - count as u8
                }
        }
    }
}

fn write_bitmap(filename: &str, pixels: &[u8], bounds: (usize, usize)) -> Result<(), std::io::Error>
{
    let output = File::create(filename)?;
    let encoder = PNGEncoder::new(output);
    encoder.encode(&pixels, bounds.0 as u32, bounds.1 as u32, ColorType::Gray(8))?;
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 6 {
        writeln!(std::io::stderr(), "Usage: mandelbrot <file> <pixels> <upperleft> <lowerright> <threads>").unwrap();
        std::process::exit(1)
    }

    let bounds = parse_pair(&args[2], 'x').expect("error parsing image dimensions");

    let upper_left = {
        let p = parse_pair(&args[3], ',').expect("error parsing upper left corner point");
        Point { x: p.0, y: p.1 }
    };

    let lower_right = {
        let p = parse_pair(&args[4], ',').expect("error parsing lower right corner point");
        Point { x: p.0, y: p.1 }
    };

    let mut pixels = vec![0; bounds.0 * bounds.1];

    let threads = usize::from_str(&args[5]).expect("error parsing thread count");

    if threads > 1 {
        println!("Parallel using {} threads.", threads);
        let band_rows = bounds.1 / threads + 1;
        let bands: Vec<&mut [u8]> = pixels.chunks_mut(band_rows * bounds.0).collect();
        crossbeam::scope(|scope| {
            for (i, band) in bands.into_iter().enumerate() {
                let top = band_rows * i;
                let height = band.len() / bounds.0;
                let band_bounds = (bounds.0, height);
                let band_upper_left = pixel_to_point(bounds, (0, top), &upper_left, &lower_right);
                let band_lower_right = pixel_to_point(bounds, (bounds.0, top + height), &upper_left, &lower_right);

                scope.spawn(move || {
                    println!(">>> Thread #{}, {} pixels", i, band.len());
                    render(band, band_bounds, &band_upper_left, &band_lower_right);
                    println!("<<< Thread #{}", i)
                });
            }
        });
    } else {
        println!("Sequential.");
        render(&mut pixels, bounds, &upper_left, &lower_right);
    }

    write_bitmap(&args[1], &pixels, bounds).expect("error writing PNG file.");
}
    type Err = ();
