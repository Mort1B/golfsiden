use super::FantasyError;
use crate::domain::scoring::handicap_strokes_for_hole;

/// The format adapter supplies its frozen individual or shared-team allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HoleSnapshot {
    pub par: i32,
    pub playing_handicap: i32,
    pub stroke_index: i32,
    pub number_of_holes: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoleInput {
    Strokes { gross: i32, snapshot: HoleSnapshot },
    Pickup,
    Pending,
    Withheld,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Ace,
    AlbatrossOrBetter,
    Eagle,
    Birdie,
    Par,
    Bogey,
    DoubleBogey,
    TripleBogey,
    QuadrupleOrWorse,
    Pickup,
}

impl Category {
    pub const fn points(self) -> i64 {
        match self {
            Self::Ace | Self::AlbatrossOrBetter => 10,
            Self::Eagle => 3,
            Self::Birdie => 1,
            Self::Par => 0,
            Self::Bogey => -1,
            Self::DoubleBogey => -2,
            Self::TripleBogey => -3,
            Self::QuadrupleOrWorse | Self::Pickup => -5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolePoints {
    Recorded(Category),
    Pending,
    Withheld,
}

fn numeric(gross: i32, snapshot: HoleSnapshot) -> Result<i64, FantasyError> {
    if gross <= 0 || snapshot.par <= 0 {
        return Err(FantasyError::InvalidFacts);
    }
    let allocated = handicap_strokes_for_hole(
        snapshot.playing_handicap,
        snapshot.stroke_index,
        snapshot.number_of_holes,
    )
    .map_err(|_| FantasyError::InvalidFacts)?;
    // Widen before subtraction: the existing hole_net_score uses i32 arithmetic.
    Ok(i64::from(gross) - i64::from(allocated) - i64::from(snapshot.par))
}

fn category(delta: i64, actual_ace: bool) -> Category {
    if actual_ace {
        return Category::Ace;
    }
    match delta {
        ..=-3 => Category::AlbatrossOrBetter,
        -2 => Category::Eagle,
        -1 => Category::Birdie,
        0 => Category::Par,
        1 => Category::Bogey,
        2 => Category::DoubleBogey,
        3 => Category::TripleBogey,
        _ => Category::QuadrupleOrWorse,
    }
}

pub fn score_hole(input: HoleInput) -> Result<HolePoints, FantasyError> {
    Ok(match input {
        HoleInput::Strokes { gross, snapshot } => {
            HolePoints::Recorded(category(numeric(gross, snapshot)?, gross == 1))
        }
        HoleInput::Pickup => HolePoints::Recorded(Category::Pickup),
        HoleInput::Pending => HolePoints::Pending,
        HoleInput::Withheld => HolePoints::Withheld,
    })
}

/// Both inputs refer to the same preserved hole. An unresolved partner may still
/// improve the counting side; do not finalize from just the other partner's score.
pub fn score_four_ball(partners: [HoleInput; 2]) -> Result<HolePoints, FantasyError> {
    if partners.contains(&HoleInput::Withheld) {
        return Ok(HolePoints::Withheld);
    }
    let mut best: Option<i64> = None;
    let mut actual_ace = false;
    let mut layout = None;
    for input in partners {
        if let HoleInput::Strokes { gross, snapshot } = input {
            let same_hole = (
                snapshot.par,
                snapshot.stroke_index,
                snapshot.number_of_holes,
            );
            if layout.is_some_and(|prior| prior != same_hole) {
                return Err(FantasyError::InvalidFacts);
            }
            layout = Some(same_hole);
            let delta = numeric(gross, snapshot)?;
            best = Some(best.map_or(delta, |value| value.min(delta)));
            actual_ace |= gross == 1;
        }
    }
    if partners.contains(&HoleInput::Pending) {
        return Ok(HolePoints::Pending);
    }
    Ok(HolePoints::Recorded(
        best.map_or(Category::Pickup, |delta| category(delta, actual_ace)),
    ))
}
