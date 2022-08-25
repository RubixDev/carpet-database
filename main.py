import os
import subprocess
import json
import re
import sys


def main():
    if len(sys.argv) > 1:
        if sys.argv[1] == '8':
            java_version = 8
        elif sys.argv[1] == '16':
            java_version = 16
        else:
            if sys.argv[1] != '18':
                print('Specified java version must be 8, 16 or 18, '
                      + 'defaulting to 18...')
            java_version = 18
    else:
        print('No java version specified, defaulting to 18...')
        java_version = 18

    with open('data/repos.json', 'r') as repos_file:
        repo_data: list[dict[str, any]] = json.load(repos_file)

    os.makedirs('repos', exist_ok=True)
    os.chdir('repos')
    failed = False
    for repo in repo_data:
        if repo['java_version'] != java_version:
            continue
        if not get_repo_data(repo):
            failed = True
    if failed:
        exit(1)


def get_repo_data(info: dict[str, any]) -> bool:
    """
    Iterates through all branches of a repo and generates the data
    file for each one
    """

    host: str = info['host']
    repo_user: str = info['repo'].split('/')[0]
    repo_name: str = info['repo'].split('/')[1]
    printer_version: int = info['printer_version']
    java_version: int = info['java_version']
    entrypoint: str = info['entrypoint'] if 'entrypoint' in info else None
    settings_manager: str = info['settings_manager'] \
        or 'carpet.CarpetServer.settingsManager'
    settings_files: list[str] = info['settings_files']
    branches: list[str] = info['branches']
    loom_override: str | None = None
    if 'loom_override' in info:
        loom_override = info['loom_override']

    if os.path.isdir(repo_name):
        os.chdir(repo_name)
    else:
        subprocess.Popen(
            ['git', 'clone', f'https://{host}/{repo_user}/{repo_name}']).wait()
        os.chdir(repo_name)

    failed = False
    for branch in branches:
        if not get_branch_data(repo_name, printer_version, java_version,
                               entrypoint, settings_manager, settings_files,
                               branch, loom_override):
            failed = True

    os.chdir('..')
    return not failed


def get_branch_data(
        repo_name: str,
        printer_version: int,
        java_version: int,
        entrypoint: str,
        settings_manager: str,
        settings_files: list[str],
        branch: str,
        loom_override: str | None) -> bool:
    """
    Generates the data file for one branch of a repo, expects
    cwd to be in the cloned repo.
    """

    print(f'\n ==> Getting data for {repo_name} on branch {branch}')
    sys.stdout.flush()

    # Undo any git changes
    subprocess.Popen(['git', 'checkout', '.']).wait()
    subprocess.Popen(['git', 'clean', '-fd']).wait()

    # Checkout branch
    subprocess.Popen(['git', 'checkout', branch]).wait()

    # Pull
    subprocess.Popen(['git', 'pull', '--no-rebase']).wait()

    # Stop if commit is still same
    if os.path.isfile(f'../../data/{repo_name}-{branch}.json'):
        with open(f'../../data/{repo_name}-{branch}.json', 'r') as data_file:
            prev_data = json.load(data_file)
        current_commit = subprocess.Popen(['git', 'rev-parse', 'HEAD'],
                                          stdout=subprocess.PIPE, text=True) \
            .stdout.read().replace('\n', '')
        if 'commit' in prev_data and prev_data['commit'] == current_commit:
            return True

    # Write Printer.java
    if printer_version == 1:
        with open('../../printers/V1Printer.java', 'r') as raw_printer_file:
            raw_printer = raw_printer_file.read()
    elif printer_version == 2:
        with open('../../printers/V2Printer.java', 'r') as raw_printer_file:
            raw_printer = raw_printer_file.read()
    else:
        print(
            f'Unsupported printer version `{printer_version}`',
            file=sys.stderr)
        return False

    printer = raw_printer.replace('SETTINGS_MANAGER', settings_manager) \
        .replace('SETTINGS_FILES', ', '.join(
            [f + '.class' for f in settings_files]))

    with open('src/main/java/Printer.java', 'w') as printer_file:
        printer_file.write(printer)

    # Set entrypoints
    with open('src/main/resources/fabric.mod.json', 'r') as fabric_file:
        fabric_conf = json.load(fabric_file)
    entrypoints = [entrypoint] if entrypoint is not None else []
    entrypoints += ['carpet.CarpetServer::onGameStarted', 'Printer::print']
    fabric_conf['entrypoints'] = {
        'main': entrypoints
    }
    with open('src/main/resources/fabric.mod.json', 'w') as fabric_file:
        json.dump(fabric_conf, fabric_file)

    # Make custom settingsManager public
    if settings_manager != 'carpet.CarpetServer.settingsManager':
        filename = 'src/main/java/' + \
            '/'.join(settings_manager.split('.')[:-1]) + '.java'
        with open(filename, 'r') as init_file:
            init_code = init_file.read()
        init_code = re.compile(
            r'private\b(.*\bstatic\b.*\bSettingsManager\b.+;)') \
            .sub(r'public\1', init_code)
        with open(filename, 'w') as init_file:
            init_file.write(init_code)

    # Get loom and gradle versions
    match java_version:
        case 8:
            loom_version = '0.7-SNAPSHOT'
            gradle_version = '6.9.2'
        case 16:
            loom_version = '0.10-SNAPSHOT'
            gradle_version = '7.4'
        case 18:
            loom_version = '0.12-SNAPSHOT'
            gradle_version = '7.4'
    if loom_override is not None:
        loom_version = loom_override

    # Set loom version and carpet maven repo and remove publishing
    with open('build.gradle', 'r') as gradle_file:
        gradle_props = gradle_file.read()
    gradle_props = re.compile(
        r"""(id\s*(['"])fabric-loom\2\s*version\s*(['"]))[^\3]+?\3""") \
        .sub(r'\g<1>' + loom_version + r'\3', gradle_props)
    gradle_props += """
repositories {
    maven { url = 'https://masa.dy.fi/maven' }
}
    """
    gradle_props = re.compile(
        r'publishing\s*\{[\s\S]*?\n\}').sub('', gradle_props)
    with open('build.gradle', 'w') as gradle_file:
        gradle_file.write(gradle_props)

    # Set gradle wrapper version
    with open('gradle/wrapper/gradle-wrapper.properties', 'r') as gradle_file:
        gradle_props = gradle_file.read()
    gradle_props = re.compile(r'(distributionUrl=.*gradle-)[^-]+(-.*)') \
        .sub(r'\g<1>' + gradle_version + r'\2', gradle_props)
    with open('gradle/wrapper/gradle-wrapper.properties', 'w') as gradle_file:
        gradle_file.write(gradle_props)

    # Accept EULA
    os.makedirs('run', exist_ok=True)
    with open('run/eula.txt', 'w') as eula_file:
        eula_file.write('eula=true')

    # Run
    os.chmod('gradlew', os.stat('gradlew').st_mode | 0o111)
    rules = subprocess.Popen(['./gradlew', 'runServer'],
                             stderr=subprocess.PIPE, text=True).stderr.read()
    if '|||DATA_START|||' not in rules:
        print(rules)
        return False
    rules = rules.split('|||DATA_START|||')[1]

    # Get current commit hash
    commit = subprocess.Popen(['git', 'rev-parse', 'HEAD'],
                              stdout=subprocess.PIPE, text=True) \
        .stdout.read().replace('\n', '')

    # Save json to file
    data = {
        'commit': commit,
        'rules': json.loads(rules),
    }
    with open(f'../../data/{repo_name}-{branch}.json', 'w') as data_file:
        json.dump(data, data_file)

    return True


if __name__ == '__main__':
    main()
