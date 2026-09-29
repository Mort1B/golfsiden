use chrono::NaiveDate;

#[derive(Debug)]
pub struct TournamentDetails {
    pub name: String,
    pub description: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

impl TournamentDetails {
    pub fn normalize(mut self) -> Result<Self, &'static str> {
        super::tournament_plan::validate_name(&self.name, "name must contain 1 to 120 bytes")?;
        if self.description.len() > 2000 || self.description.contains('\0') {
            return Err("description must not exceed 2000 bytes");
        }
        if self.start_date > self.end_date {
            return Err("end_date must not be before start_date");
        }
        self.name = self.name.trim().to_owned();
        self.description = self.description.trim().to_owned();
        Ok(self)
    }
}
