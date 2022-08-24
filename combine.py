import json


def is_sublist(list1: list, list2: list) -> bool:
    return list1 in [list2[i:len(list1) + i] for i in range(len(list2))]


with open('data/repos.json', 'r') as repos_file:
    repo_info = json.load(repos_file)

data = []

for repo in repo_info:
    repo_name = repo['repo'].split('/')[1]
    for branch in repo['branches']:
        with open(f'data/{repo_name}-{branch}.json', 'r') as branch_file:
            branch_data = json.load(branch_file)
        for new_rule in branch_data['rules']:
            new_rule['host'] = repo['host']
            new_rule['repo'] = repo['repo']
            new_rule['config_files'] = repo['config_files']
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
                        and (
                            is_sublist(rule['validators'],
                                       new_rule['validators'])
                            or is_sublist(new_rule['validators'],
                                          rule['validators'])
                        ) \
                        and rule['host'] == new_rule['host'] \
                        and rule['repo'] == new_rule['repo'] \
                        and rule['config_files'] == new_rule['config_files']:
                    if len(new_rule['validators']) > len(rule['validators']):
                        rule['validators'] = new_rule['validators']
                    rule['branches'].append(branch)
                    did_modify = True
            if not did_modify:
                data.append(new_rule)

for rule in data:
    if rule['extras'] == []:
        del rule['extras']
    if rule['validators'] == []:
        del rule['validators']

# Print rule count stats
rule_counts = {}
total_count = 0
for repo in repo_info:
    if repo['repo'] in rule_counts:
        continue
    count = len(set(
        rule['name'] for rule in data if rule['repo'] == repo['repo']))
    rule_counts[repo['repo']] = count
    total_count += count

print(f'**Rules parsed**: {total_count}\n')
for repo, count in rule_counts.items():
    print(f'**{repo}**: {count}')

with open('data/combined.json', 'w') as data_file:
    json.dump(data, data_file)
