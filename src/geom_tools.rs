use crate::prelude::*;

// TODO Clear code
pub struct Line {
    start: IVec2,
    current: IVec2,
    end: IVec2,
    straight: bool,
    hor: bool,
    tan: f32,
    max: f32,
}

impl Line {
    #[inline]
    pub fn new(start: IVec2, vec: IVec2) -> Self {
        // Get end position
        let end = start + vec;

        let dx = end.x - start.x;
        let dy = end.y - start.y;

        let hor = (end.x - start.x).abs() > (end.y - start.y).abs();
        let tan = if hor {
            (end.y - start.y) as f32 / (end.x - start.x) as f32
        } else {
            (end.x - start.x) as f32 / (end.y - start.y) as f32 // Technically cotangent
        };
        let max = (1. - tan.abs()) / 2.;

        Self {
            start,
            current: start,
            end,
            straight: dx.signum() == 0 || dy.signum() == 0,
            hor,
            tan,
            max,
        }
    }
}

impl Iterator for Line {
    type Item = IVec2;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let dx = self.end.x - self.start.x;
        let dy = self.end.y - self.start.y;

        if self.current.x == self.end.x && self.current.y == self.end.y {
            return None;
        }

        if self.straight {
            self.current.y += dy.signum();
            self.current.x += dx.signum();

            return Some(self.current);
        }

        if self.hor {
            let ideal = self.start.y as f32 + (self.current.x - self.start.x) as f32 * self.tan;

            if (ideal - self.current.y as f32) * dy.signum() as f32 >= self.max {
                self.current.y += dy.signum()
            } else {
                self.current.x += dx.signum()
            }
        } else {
            let ideal = self.start.x as f32 + (self.current.y - self.start.y) as f32 * self.tan;

            if (ideal - self.current.x as f32) * dx.signum() as f32 >= self.max {
                self.current.x += dx.signum()
            } else {
                self.current.y += dy.signum()
            }
        }

        Some(self.current)
    }
}

// Gonna maybe be used for fluid sim in the future
pub fn _circle_points(center: IVec2, radius: i32) -> Vec<IVec2> {
    let mut points = vec![];

    let mut x = center.x - radius;
    let mut y = center.y - radius;

    while x <= center.x {
        while y <= center.y {
            if (x - center.x).pow(2) + (y - center.y).pow(2) <= radius.pow(2) {
                let x_sym = center.x - (x - center.x);
                let y_sym = center.y - (y - center.y);

                points.push(IVec2::new(x, y));
                points.push(IVec2::new(x, y_sym));
                points.push(IVec2::new(x_sym, y));
                points.push(IVec2::new(x_sym, y_sym));
            }
            y += 1;
        }
        x += 1;
    }

    points
}
