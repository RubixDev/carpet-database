import toml
import json

with open('repos.toml', 'r') as toml_file:
    repos = toml.load(toml_file)['repos']

for repo in repos:
    if 'entrypoint' not in repo:
        repo['entrypoint'] = None
    if 'settings_manager' not in repo:
        repo['settings_manager'] = None

with open('data/repos.json', 'w') as json_file:
    json.dump(repos, json_file)
