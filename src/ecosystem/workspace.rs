//! Workspace Support Enhancements
//!
//! Support for multi-crate workspaces with member management

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Workspace configuration
#[derive(Debug, Clone)]
pub struct WorkspaceConfig {
    pub resolver: String,
    pub members: Vec<String>,
    pub exclude: Vec<String>,
    pub default_members: Vec<String>,
}

/// Workspace member
#[derive(Debug, Clone)]
pub struct WorkspaceMember {
    pub name: String,
    pub path: PathBuf,
    pub version: String,
    pub kind: MemberKind,
}

/// Type of workspace member
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberKind {
    Binary,
    Library,
    Proc,
    Example,
}

impl MemberKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemberKind::Binary => "bin",
            MemberKind::Library => "lib",
            MemberKind::Proc => "proc-macro",
            MemberKind::Example => "example",
        }
    }
}

/// Workspace
pub struct Workspace {
    root: PathBuf,
    config: WorkspaceConfig,
    members: HashMap<String, WorkspaceMember>,
    dependencies: HashMap<String, Vec<String>>,  // member -> dependencies
}

impl Workspace {
    /// Create new workspace
    pub fn new(root: PathBuf, config: WorkspaceConfig) -> Self {
        Workspace {
            root,
            config,
            members: HashMap::new(),
            dependencies: HashMap::new(),
        }
    }

    /// Load workspace from path
    pub fn load<P: AsRef<Path>>(root: P) -> Result<Self, String> {
        let root = root.as_ref().to_path_buf();

        let config = WorkspaceConfig {
            resolver: "2".to_string(),
            members: Vec::new(),
            exclude: Vec::new(),
            default_members: Vec::new(),
        };

        Ok(Workspace::new(root, config))
    }

    /// Add member to workspace
    pub fn add_member(&mut self, member: WorkspaceMember) -> Result<(), String> {
        if self.members.contains_key(&member.name) {
            return Err(format!("Member {} already exists", member.name));
        }

        let name = member.name.clone();
        self.members.insert(name.clone(), member);
        self.dependencies.insert(name, Vec::new());

        Ok(())
    }

    /// Remove member from workspace
    pub fn remove_member(&mut self, name: &str) -> Result<WorkspaceMember, String> {
        self.members
            .remove(name)
            .ok_or_else(|| format!("Member {} not found", name))
    }

    /// Get member
    pub fn get_member(&self, name: &str) -> Option<&WorkspaceMember> {
        self.members.get(name)
    }

    /// Get all members
    pub fn get_members(&self) -> Vec<&WorkspaceMember> {
        self.members.values().collect()
    }

    /// Get members by kind
    pub fn get_members_by_kind(&self, kind: MemberKind) -> Vec<&WorkspaceMember> {
        self.members
            .values()
            .filter(|m| m.kind == kind)
            .collect()
    }

    /// Add dependency between members
    pub fn add_dependency(&mut self, from: &str, to: String) -> Result<(), String> {
        if !self.members.contains_key(from) {
            return Err(format!("Member {} not found", from));
        }

        if let Some(deps) = self.dependencies.get_mut(from) {
            if !deps.contains(&to) {
                deps.push(to);
            }
        }

        Ok(())
    }

    /// Get dependencies of member
    pub fn get_dependencies(&self, name: &str) -> Option<Vec<&String>> {
        self.dependencies.get(name).map(|deps| {
            deps.iter().collect()
        })
    }

    /// Get workspace root
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Get resolver version
    pub fn resolver(&self) -> &str {
        &self.config.resolver
    }

    /// Get default members
    pub fn default_members(&self) -> &[String] {
        &self.config.default_members
    }

    /// Check if member exists
    pub fn has_member(&self, name: &str) -> bool {
        self.members.contains_key(name)
    }

    /// Get member count
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    /// Verify workspace consistency
    pub fn verify(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Check that dependencies point to existing members
        for (member, deps) in &self.dependencies {
            for dep in deps {
                if !self.members.contains_key(dep) {
                    errors.push(format!(
                        "Member {} depends on non-existent member {}",
                        member, dep
                    ));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Get build order (topological sort)
    pub fn build_order(&self) -> Result<Vec<String>, String> {
        let mut visited = std::collections::HashSet::new();
        let mut order = Vec::new();

        for member in self.members.keys() {
            self.visit_member(member, &mut visited, &mut order)?;
        }

        Ok(order)
    }

    fn visit_member(
        &self,
        name: &str,
        visited: &mut std::collections::HashSet<String>,
        order: &mut Vec<String>,
    ) -> Result<(), String> {
        if visited.contains(name) {
            return Ok(());
        }

        visited.insert(name.to_string());

        // Visit dependencies first
        if let Some(deps) = self.dependencies.get(name) {
            for dep in deps {
                self.visit_member(dep, visited, order)?;
            }
        }

        order.push(name.to_string());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_creation() {
        let config = WorkspaceConfig {
            resolver: "2".to_string(),
            members: vec!["lib".to_string(), "bin".to_string()],
            exclude: vec![],
            default_members: vec!["lib".to_string()],
        };
        let ws = Workspace::new(PathBuf::from("/project"), config);
        assert_eq!(ws.root(), Path::new("/project"));
    }

    #[test]
    fn test_add_member() {
        let config = WorkspaceConfig {
            resolver: "2".to_string(),
            members: vec![],
            exclude: vec![],
            default_members: vec![],
        };
        let mut ws = Workspace::new(PathBuf::from("/project"), config);

        let member = WorkspaceMember {
            name: "mylib".to_string(),
            path: PathBuf::from("/project/mylib"),
            version: "0.1.0".to_string(),
            kind: MemberKind::Library,
        };

        let result = ws.add_member(member);
        assert!(result.is_ok());
        assert!(ws.has_member("mylib"));
    }

    #[test]
    fn test_add_duplicate_member() {
        let config = WorkspaceConfig {
            resolver: "2".to_string(),
            members: vec![],
            exclude: vec![],
            default_members: vec![],
        };
        let mut ws = Workspace::new(PathBuf::from("/project"), config);

        let member = WorkspaceMember {
            name: "duplicate".to_string(),
            path: PathBuf::from("/project/duplicate"),
            version: "0.1.0".to_string(),
            kind: MemberKind::Library,
        };

        ws.add_member(member.clone()).unwrap();
        let result = ws.add_member(member);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_members_by_kind() {
        let config = WorkspaceConfig {
            resolver: "2".to_string(),
            members: vec![],
            exclude: vec![],
            default_members: vec![],
        };
        let mut ws = Workspace::new(PathBuf::from("/project"), config);

        ws.add_member(WorkspaceMember {
            name: "lib".to_string(),
            path: PathBuf::from("/project/lib"),
            version: "0.1.0".to_string(),
            kind: MemberKind::Library,
        })
        .unwrap();

        ws.add_member(WorkspaceMember {
            name: "bin".to_string(),
            path: PathBuf::from("/project/bin"),
            version: "0.1.0".to_string(),
            kind: MemberKind::Binary,
        })
        .unwrap();

        let libs = ws.get_members_by_kind(MemberKind::Library);
        assert_eq!(libs.len(), 1);
    }

    #[test]
    fn test_add_dependency() {
        let config = WorkspaceConfig {
            resolver: "2".to_string(),
            members: vec![],
            exclude: vec![],
            default_members: vec![],
        };
        let mut ws = Workspace::new(PathBuf::from("/project"), config);

        ws.add_member(WorkspaceMember {
            name: "lib".to_string(),
            path: PathBuf::from("/project/lib"),
            version: "0.1.0".to_string(),
            kind: MemberKind::Library,
        })
        .unwrap();

        ws.add_member(WorkspaceMember {
            name: "bin".to_string(),
            path: PathBuf::from("/project/bin"),
            version: "0.1.0".to_string(),
            kind: MemberKind::Binary,
        })
        .unwrap();

        let result = ws.add_dependency("bin", "lib".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_order() {
        let config = WorkspaceConfig {
            resolver: "2".to_string(),
            members: vec![],
            exclude: vec![],
            default_members: vec![],
        };
        let mut ws = Workspace::new(PathBuf::from("/project"), config);

        ws.add_member(WorkspaceMember {
            name: "lib".to_string(),
            path: PathBuf::from("/project/lib"),
            version: "0.1.0".to_string(),
            kind: MemberKind::Library,
        })
        .unwrap();

        ws.add_member(WorkspaceMember {
            name: "bin".to_string(),
            path: PathBuf::from("/project/bin"),
            version: "0.1.0".to_string(),
            kind: MemberKind::Binary,
        })
        .unwrap();

        ws.add_dependency("bin", "lib".to_string()).unwrap();

        let order = ws.build_order();
        assert!(order.is_ok());
        let order = order.unwrap();
        let lib_idx = order.iter().position(|m| m == "lib");
        let bin_idx = order.iter().position(|m| m == "bin");
        assert!(lib_idx < bin_idx);
    }

    #[test]
    fn test_member_kind_as_str() {
        assert_eq!(MemberKind::Binary.as_str(), "bin");
        assert_eq!(MemberKind::Library.as_str(), "lib");
        assert_eq!(MemberKind::Proc.as_str(), "proc-macro");
    }

    #[test]
    fn test_workspace_member_count() {
        let config = WorkspaceConfig {
            resolver: "2".to_string(),
            members: vec![],
            exclude: vec![],
            default_members: vec![],
        };
        let mut ws = Workspace::new(PathBuf::from("/project"), config);

        ws.add_member(WorkspaceMember {
            name: "member1".to_string(),
            path: PathBuf::from("/project/member1"),
            version: "0.1.0".to_string(),
            kind: MemberKind::Library,
        })
        .unwrap();

        assert_eq!(ws.member_count(), 1);
    }
}
