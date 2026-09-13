use serde::Serialize;
use uuid::Uuid;
#[derive(Serialize, sqlx::FromRow)]
pub struct Participant {
    pub player_id: Uuid,
    pub display_name: String,
}
#[derive(sqlx::FromRow)]
pub struct AwardFact {
    pub first_player_id: Uuid,
    pub second_player_id: Uuid,
    pub first_half_points: i16,
    pub second_half_points: i16,
}
#[derive(Debug, Serialize)]
pub struct TableEntry {
    pub player_id: Uuid,
    pub display_name: String,
    pub half_points: u32,
    pub played: u32,
    pub wins: u32,
    pub draws: u32,
    pub losses: u32,
    pub position: Option<usize>,
}
pub fn assemble(
    participants: Vec<Participant>,
    awards: Vec<AwardFact>,
) -> Result<Vec<TableEntry>, &'static str> {
    let mut entries = participants
        .into_iter()
        .map(|p| TableEntry {
            player_id: p.player_id,
            display_name: p.display_name,
            half_points: 0,
            played: 0,
            wins: 0,
            draws: 0,
            losses: 0,
            position: None,
        })
        .collect::<Vec<_>>();
    let indexes = entries
        .iter()
        .enumerate()
        .map(|(i, p)| (p.player_id, i))
        .collect::<std::collections::HashMap<_, _>>();
    if indexes.len() != entries.len() {
        return Err("duplicate table participant");
    }
    for award in awards {
        if award.first_player_id == award.second_player_id
            || !(0..=2).contains(&award.first_half_points)
            || award.second_half_points != 2 - award.first_half_points
        {
            return Err("invalid match point award");
        }
        for (id, points) in [
            (award.first_player_id, award.first_half_points),
            (award.second_player_id, award.second_half_points),
        ] {
            let entry = indexes
                .get(&id)
                .and_then(|i| entries.get_mut(*i))
                .ok_or("missing match participant")?;
            entry.half_points = entry
                .half_points
                .checked_add(points as u32)
                .ok_or("match points overflow")?;
            entry.played = entry.played.checked_add(1).ok_or("played overflow")?;
            match points {
                2 => entry.wins += 1,
                1 => entry.draws += 1,
                _ => entry.losses += 1,
            }
        }
    }
    entries.sort_by(|a, b| {
        (b.played > 0)
            .cmp(&(a.played > 0))
            .then(b.half_points.cmp(&a.half_points))
            .then_with(|| {
                a.display_name
                    .to_lowercase()
                    .cmp(&b.display_name.to_lowercase())
            })
            .then(a.display_name.cmp(&b.display_name))
            .then(a.player_id.cmp(&b.player_id))
    });
    let mut prior = None;
    for (i, entry) in entries.iter_mut().enumerate() {
        if entry.played > 0 {
            let place = match prior {
                Some((points, place)) if points == entry.half_points => place,
                _ => i + 1,
            };
            entry.position = Some(place);
            prior = Some((entry.half_points, place));
        }
    }
    Ok(entries)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_units_shared_places_and_unranked() {
        let entries = assemble(
            (1..=4)
                .map(|i| Participant {
                    player_id: Uuid::from_u128(i),
                    display_name: format!("P{i}"),
                })
                .collect(),
            vec![AwardFact {
                first_player_id: Uuid::from_u128(1),
                second_player_id: Uuid::from_u128(2),
                first_half_points: 1,
                second_half_points: 1,
            }],
        )
        .unwrap();
        assert_eq!(entries[0].half_points, 1);
        assert_eq!(entries[0].position, Some(1));
        assert_eq!(entries[1].position, Some(1));
        assert_eq!(entries[2].position, None);
        assert_eq!(entries[0].draws, 1);
    }
}
