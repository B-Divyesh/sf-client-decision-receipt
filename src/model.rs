use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LineItem {
    pub label: String,
    #[serde(default)]
    pub description: String,
    pub quantity: i64,
    pub unit_amount_cents: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProposal {
    pub title: String,
    pub freelancer_name: String,
    pub freelancer_email: String,
    pub client_name: String,
    pub client_email: String,
    #[serde(default)]
    pub message: String,
    pub currency: String,
    pub items: Vec<LineItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Decision {
    pub kind: String,
    pub respondent_name: String,
    pub respondent_email: String,
    pub note: String,
    pub decided_at: String,
    pub receipt_hash: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Proposal {
    pub id: String,
    pub title: String,
    pub freelancer_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freelancer_email: Option<String>,
    pub client_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_email: Option<String>,
    pub message: String,
    pub currency: String,
    pub items: Vec<LineItem>,
    pub created_at: String,
    pub decision: Option<Decision>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitDecision {
    pub kind: String,
    pub respondent_name: String,
    pub respondent_email: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedProposal {
    pub id: String,
    pub client_path: String,
    pub manage_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedProposal {
    #[serde(flatten)]
    pub proposal: Proposal,
    pub client_path: String,
    pub delivery: Vec<Delivery>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Delivery {
    pub recipient: String,
    pub status: String,
    pub created_at: String,
}

pub fn validate_create(input: &CreateProposal) -> Result<(), &'static str> {
    text(&input.title, 3, 120, "Proposal title must be 3–120 characters.")?;
    text(&input.freelancer_name, 2, 80, "Your name must be 2–80 characters.")?;
    text(&input.client_name, 2, 80, "Client name must be 2–80 characters.")?;
    email(&input.freelancer_email)?;
    email(&input.client_email)?;
    if input.message.chars().count() > 2000 { return Err("Introduction must be at most 2,000 characters."); }
    if input.currency.len() != 3 || !input.currency.chars().all(|c| c.is_ascii_uppercase()) { return Err("Currency must be a three-letter code such as USD."); }
    if input.items.is_empty() || input.items.len() > 30 { return Err("Add between 1 and 30 scope items."); }
    for item in &input.items {
        text(&item.label, 2, 160, "Each scope item needs a 2–160 character title.")?;
        if item.description.chars().count() > 500 { return Err("Scope item details must be at most 500 characters."); }
        if !(1..=999).contains(&item.quantity) { return Err("Quantity must be between 1 and 999."); }
        if !(0..=100_000_000).contains(&item.unit_amount_cents) { return Err("Item price is outside the supported range."); }
    }
    Ok(())
}

pub fn validate_decision(input: &SubmitDecision) -> Result<(), &'static str> {
    if !matches!(input.kind.as_str(), "accepted" | "changes_requested" | "declined") { return Err("Choose Accept, Request changes, or Decline."); }
    text(&input.respondent_name, 2, 80, "Your name must be 2–80 characters.")?;
    email(&input.respondent_email)?;
    if input.note.chars().count() > 2000 { return Err("Decision note must be at most 2,000 characters."); }
    if input.kind == "changes_requested" && input.note.trim().len() < 3 { return Err("Describe the changes you need."); }
    Ok(())
}

fn text(value: &str, min: usize, max: usize, error: &'static str) -> Result<(), &'static str> {
    let len = value.trim().chars().count();
    if len < min || len > max { Err(error) } else { Ok(()) }
}

fn email(value: &str) -> Result<(), &'static str> {
    let value = value.trim();
    let valid = value.len() <= 254 && value.split_once('@').is_some_and(|(local, domain)| !local.is_empty() && domain.contains('.') && !domain.contains(char::is_whitespace));
    if valid { Ok(()) } else { Err("Enter a valid email address.") }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn change_request_requires_note() {
        let input = SubmitDecision { kind: "changes_requested".into(), respondent_name: "A Client".into(), respondent_email: "a@example.com".into(), note: "".into() };
        assert_eq!(validate_decision(&input), Err("Describe the changes you need."));
    }
}
