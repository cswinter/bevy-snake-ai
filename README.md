# how to deploy web build

- on main branch, run `trunk build --release`
- copy the dist directory (and assets directory if changed) onto web branch
- in the generated index.html, prefix all urls with /bevy-snake-ai