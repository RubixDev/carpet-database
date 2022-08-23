import json

with open('data/repos.json', 'r') as repos_file:
    repo_info = json.load(repos_file)

data = []

for repo in repo_info:
    repo_name = repo['repo'].split('/')[1]
    for branch in repo['branches']:
        with open(f'data/{repo_name}-{branch}.json', 'r') as branch_file:
            branch_data = json.load(branch_file)
        for new_rule in branch_data:
            new_rule['repo'] = repo['repo']
            new_rule['branches'] = [branch]

            did_modify = False
            for rule in data:
                if rule['name'] == new_rule['name'] \
                        and rule['description'] == new_rule['description'] \
                        and rule['type'] == new_rule['type'] \
                        and rule['value'] == new_rule['value'] \
                        and rule['strict'] == new_rule['strict'] \
                        and rule['categories'] == new_rule['categories'] \
                        and rule['options'] == new_rule['options'] \
                        and rule['extras'] == new_rule['extras'] \
                        and rule['validators'] == new_rule['validators'] \
                        and rule['repo'] == new_rule['repo']:
                    rule['branches'].append(branch)
                    did_modify = True
            if not did_modify:
                data.append(new_rule)

with open('data/combined.json', 'w') as data_file:
    json.dump(data, data_file)
