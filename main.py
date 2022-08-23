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
    for repo in repo_data:
        if repo['java_version'] != java_version:
            continue
        get_repo_data(repo)


def get_repo_data(info: dict[str, any]) -> dict[str, any]:
    """
    Iterates through all branches of a repo and generates the data
    file for each one
    """

    host: str = info['host']
    repo_user: str = info['repo'].split('/')[0]
    repo_name: str = info['repo'].split('/')[1]
    printer_version: int = info['printer_version']
    java_version: int = info['java_version']
    entrypoint: str = info['entrypoint']
    settings_manager: str = info['settings_manager']
    branches: list[str] = info['branches']

    if os.path.isdir(repo_name):
        os.chdir(repo_name)
    else:
        subprocess.Popen(
            ['git', 'clone', f'https://{host}/{repo_user}/{repo_name}']).wait()
        os.chdir(repo_name)

    for branch in branches:
        get_branch_data(repo_name, printer_version, java_version, entrypoint,
                        settings_manager, branch)

    os.chdir('..')


def get_branch_data(
        repo_name: str,
        printer_version: int,
        java_version: int,
        entrypoint: str,
        settings_manager: str,
        branch: str):
    """
    Generates the data file for one branch of a repo, expects
    cwd to be in the cloned repo.
    """

    # Undo any git changes
    subprocess.Popen(['git', 'checkout', '.']).wait()
    subprocess.Popen(['git', 'clean', '-fd']).wait()

    # Checkout branch
    subprocess.Popen(['git', 'checkout', branch]).wait()

    # Pull
    subprocess.Popen(['git', 'pull', '--no-rebase']).wait()

    # Write Printer.java
    if printer_version == 1:
        with open('../../printers/V1Printer.java', 'r') as raw_printer_file:
            raw_printer = raw_printer_file.read()
    elif printer_version == 2:
        print('TODO')  # TODO: v2 Printer
    else:
        print(
            f'Unsupported printer version `{printer_version}`',
            file=sys.stderr)
        return

    printer = raw_printer.replace('SETTINGS_MANAGER', settings_manager)
    with open('src/main/java/Printer.java', 'w') as printer_file:
        printer_file.write(printer)

    # Set entrypoints
    with open('src/main/resources/fabric.mod.json', 'r') as fabric_file:
        fabric_conf = json.load(fabric_file)
    fabric_conf['entrypoints']['main'] = [entrypoint, 'Printer::print']
    with open('src/main/resources/fabric.mod.json', 'w') as fabric_file:
        json.dump(fabric_conf, fabric_file)

    # Make custom settingsManager public
    if settings_manager != 'carpet.CarpetServer.settingsManager':
        filename = 'src/main/java/' + \
            '/'.join(settings_manager.split('.')[:-1]) + '.java'
        with open(filename, 'r') as init_file:
            init_code = init_file.read()
        init_code = re.compile(
            r'private (static SettingsManager \w+;)') \
            .sub(r'public \1', init_code)
        with open(filename, 'w') as init_file:
            init_file.write(init_code)

    # Set loom version
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
    with open('build.gradle', 'r') as gradle_file:
        gradle_props = gradle_file.read()
    gradle_props = re.compile(
        r"""(id\s*(['"])fabric-loom\2\s*version\s*(['"]))[^\3]+?\3""") \
        .sub(r'\g<1>' + loom_version + r'\3', gradle_props)
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
    data = subprocess.Popen(['./gradlew', 'runServer'],
                            stderr=subprocess.PIPE, text=True).stderr.read()

    if '|||DATA_START|||' not in data:
        print(data)
        return
    # Save json to file
    with open(f'../../data/{repo_name}-{branch}.json', 'w') as data_file:
        data_file.write(data.split('|||DATA_START|||')[1])


if __name__ == '__main__':
    main()
