## Plan: Restructure by Subject (Env/FS/Domain)

Split large “verb” modules into smaller “subject” modules: shared filesystem/layout utilities (home/project roots), preset naming + IO + discovery, and preset resolution submodules (vars/paths/commands/inheritance/settings). This reduces file size, prevents “util”/“loader”/“resolver” from becoming dumping grounds, and fixes unhealthy dependencies (notably settings relying on preset loader). Keep behavior stable by first extracting helpers behind existing entry points, then splitting internals, then tightening dependency direction.

### Steps
1. Create shared path/layout modules in claudio-core: [crates/claudio-core/src/paths](crates/claudio-core/src/paths) for `get_claudio_home` and `find_project_root`.
2. Refactor preset “loader” into subject modules: naming in [crates/claudio-core/src/preset/naming.rs](crates/claudio-core/src/preset/naming.rs), IO in [crates/claudio-core/src/preset/io.rs](crates/claudio-core/src/preset/io.rs), discovery in [crates/claudio-core/src/preset/discovery.rs](crates/claudio-core/src/preset/discovery.rs).
3. Split resolver into focused submodules under [crates/claudio-core/src/preset/resolve](crates/claudio-core/src/preset/resolve): `vars`, `paths`, `command`, `env`, `settings`, `inheritance`.
4. Remove settings→preset coupling by moving path/root logic out of [crates/claudio-core/src/preset/loader.rs](crates/claudio-core/src/preset/loader.rs) and into shared paths; update [crates/claudio-core/src/settings/loader.rs](crates/claudio-core/src/settings/loader.rs) to call shared modules directly.
5. Tighten resolver dependencies: stop importing preset loader from resolver; route base preset lookup through a store/lookup interface owned by preset store (or an adapter), keeping inheritance logic isolated from disk lookup.

### Further Considerations
1. Should `~` expansion respect `CLAUDIO_HOME_DIR`? Option A: keep OS home (status quo). Option B: use `get_claudio_home`.
2. If breaking changes are acceptable, rename [crates/claudio-core/src/util.rs](crates/claudio-core/src/util.rs) to `paths/home.rs` and update imports immediately.
3. Decide whether “env” means env-var parsing only, or also `${VAR}` substitution used by presets/settings.
