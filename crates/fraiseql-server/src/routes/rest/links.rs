//! HAL-style `_links` builder for single-resource responses.

use fraiseql_core::schema::{Cardinality, Relationship};
use serde_json::{json, Value};

use super::resource::RestRouteTable;

/// Builds HAL-style `_links` JSON for a single-resource response.
pub struct HalLinkBuilder<'a> {
    base_path: &'a str,
    route_table: &'a RestRouteTable,
}

impl<'a> HalLinkBuilder<'a> {
    /// Create a new link builder with the given base path and route table.
    pub fn new(base_path: &'a str, route_table: &'a RestRouteTable) -> Self {
        Self {
            base_path,
            route_table,
        }
    }

    /// Build `_links` for a resource instance.
    ///
    /// - `resource_path`: the collection path (e.g., `/candidates`)
    /// - `id_value`: the entity's ID as a string
    /// - `relationships`: the type's relationship metadata
    pub fn build(
        &self,
        resource_path: &str,
        id_value: &str,
        relationships: &[Relationship],
    ) -> Value {
        let self_href = format!("{}{}/{}", self.base_path, resource_path, id_value);
        let collection_href = format!("{}{}", self.base_path, resource_path);

        let mut links = serde_json::Map::new();
        links.insert("self".to_string(), json!({"href": self_href}));
        links.insert("collection".to_string(), json!({"href": collection_href}));

        let mut children = serde_json::Map::new();
        let mut related = serde_json::Map::new();

        for rel in relationships {
            let target_resource = match self.route_table.find_resource_by_type(&rel.target_type) {
                Some(r) => r,
                None => continue,
            };

            let target_path = format!("{}/{}", self.base_path, target_resource.name);

            match rel.cardinality {
                Cardinality::OneToMany => {
                    let href =
                        format!("{}?filter={}.eq.{}", target_path, rel.foreign_key, id_value);
                    children.insert(rel.name.clone(), json!({"href": href}));
                }
                Cardinality::ManyToOne | Cardinality::OneToOne | _ => {
                    related.insert(rel.name.clone(), json!({"href": target_path}));
                }
            }
        }

        if !children.is_empty() {
            links.insert("children".to_string(), Value::Object(children));
        }
        if !related.is_empty() {
            links.insert("related".to_string(), Value::Object(related));
        }

        Value::Object(links)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::resource::{RestResource, RestRouteTable};
    use super::*;

    fn make_route_table() -> RestRouteTable {
        RestRouteTable {
            base_path: "/rest/v1".to_string(),
            resources: vec![
                RestResource {
                    name: "candidates".to_string(),
                    type_name: "Candidate".to_string(),
                    id_arg: Some("id".to_string()),
                    routes: vec![],
                },
                RestResource {
                    name: "profiles".to_string(),
                    type_name: "CandidateProfile".to_string(),
                    id_arg: Some("id".to_string()),
                    routes: vec![],
                },
                RestResource {
                    name: "applications".to_string(),
                    type_name: "Application".to_string(),
                    id_arg: Some("id".to_string()),
                    routes: vec![],
                },
            ],
            diagnostics: vec![],
        }
    }

    #[test]
    fn build_includes_self_link() {
        let table = make_route_table();
        let builder = HalLinkBuilder::new("/rest/v1", &table);
        let links = builder.build("/candidates", "abc-123", &[]);
        assert_eq!(links["self"]["href"], "/rest/v1/candidates/abc-123");
    }

    #[test]
    fn build_includes_collection_link() {
        let table = make_route_table();
        let builder = HalLinkBuilder::new("/rest/v1", &table);
        let links = builder.build("/candidates", "abc-123", &[]);
        assert_eq!(links["collection"]["href"], "/rest/v1/candidates");
    }

    #[test]
    fn build_includes_one_to_many_children() {
        let table = make_route_table();
        let builder = HalLinkBuilder::new("/rest/v1", &table);
        let rels = vec![Relationship {
            name: "profiles".to_string(),
            target_type: "CandidateProfile".to_string(),
            cardinality: Cardinality::OneToMany,
            foreign_key: "fk_candidate".to_string(),
            referenced_key: "id".to_string(),
        }];
        let links = builder.build("/candidates", "abc-123", &rels);
        assert_eq!(
            links["children"]["profiles"]["href"],
            "/rest/v1/profiles?filter=fk_candidate.eq.abc-123"
        );
    }

    #[test]
    fn build_includes_many_to_one_as_related() {
        let table = make_route_table();
        let builder = HalLinkBuilder::new("/rest/v1", &table);
        let rels = vec![Relationship {
            name: "applications".to_string(),
            target_type: "Application".to_string(),
            cardinality: Cardinality::ManyToOne,
            foreign_key: "fk_application".to_string(),
            referenced_key: "id".to_string(),
        }];
        let links = builder.build("/candidates", "abc-123", &rels);
        assert_eq!(
            links["related"]["applications"]["href"],
            "/rest/v1/applications"
        );
    }

    #[test]
    fn build_skips_relationships_with_no_matching_resource() {
        let table = RestRouteTable {
            base_path: "/rest/v1".to_string(),
            resources: vec![RestResource {
                name: "candidates".to_string(),
                type_name: "Candidate".to_string(),
                id_arg: Some("id".to_string()),
                routes: vec![],
            }],
            diagnostics: vec![],
        };
        let builder = HalLinkBuilder::new("/rest/v1", &table);
        let rels = vec![Relationship {
            name: "unknown".to_string(),
            target_type: "NonExistent".to_string(),
            cardinality: Cardinality::OneToMany,
            foreign_key: "fk_x".to_string(),
            referenced_key: "id".to_string(),
        }];
        let links = builder.build("/candidates", "abc-123", &rels);
        assert!(links.get("children").is_none());
    }

    #[test]
    fn build_multiple_children_and_related() {
        let table = make_route_table();
        let builder = HalLinkBuilder::new("/rest/v1", &table);
        let rels = vec![
            Relationship {
                name: "profiles".to_string(),
                target_type: "CandidateProfile".to_string(),
                cardinality: Cardinality::OneToMany,
                foreign_key: "fk_candidate".to_string(),
                referenced_key: "id".to_string(),
            },
            Relationship {
                name: "applications".to_string(),
                target_type: "Application".to_string(),
                cardinality: Cardinality::ManyToOne,
                foreign_key: "fk_application".to_string(),
                referenced_key: "id".to_string(),
            },
        ];
        let links = builder.build("/candidates", "abc-123", &rels);
        assert!(links["self"]["href"].is_string());
        assert!(links["children"]["profiles"]["href"].is_string());
        assert!(links["related"]["applications"]["href"].is_string());
    }
}
