use crate::topology::types::ZoneGroup;

impl ZoneGroup {
    pub fn get_name(&self) -> &str {
        self.members
            .first()
            .map(|member| member.zone_name.as_str())
            .unwrap_or("Group")
    }
}