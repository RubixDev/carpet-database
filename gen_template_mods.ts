import { ensureDir } from 'https://deno.land/std@0.177.1/fs/ensure_dir.ts'
import * as path from 'https://deno.land/std@0.177.1/path/mod.ts'
import { colors } from 'https://deno.land/x/cliffy@v0.25.7/ansi/mod.ts'
import { parse as parseXml } from 'https://deno.land/x/xml@2.1.1/mod.ts'
import * as generator from './tmp/fabricmc.net/scripts/dist/fabric-template-generator.js'

const error = colors.bold.red
const info = colors.blue
const trace = colors.gray

// Set the XML parser as we do not have DomParser here.
generator.setXmlVersionParser(xml => {
    const document = parseXml(xml) as any
    return document.metadata.versioning.versions.version
})

await generate()

async function isDirEmpty(outputDir: string): Promise<boolean> {
    const contents = Deno.readDir(outputDir)

    for await (const _ of contents) {
        return false
    }

    return true
}

async function generate() {
    for (const version of Deno.args) {
        console.log(info(`> generating mod template for '${version}'`))
        const outputDir = await getAndPrepareOutputDir('tmp/templates/' + version)

        const isTargetEmpty = await isDirEmpty(outputDir)
        if (!isTargetEmpty) {
            console.log(trace('directory already exists, skipping'))
            continue
        }

        const config = {
            modid: 'data-extractor',
            minecraftVersion: version,
            projectName: 'data-extractor',
            packageName: 'com.example',
            useKotlin: false,
            dataGeneration: false,
            splitSources: false,
        }

        const options: generator.Options = {
            config,
            writer: {
                write: async (contentPath, content, options) => {
                    await writeFile(outputDir, contentPath, content, options)
                },
            },
        }

        await generator.generateTemplate(options)
    }
}

async function getAndPrepareOutputDir(
    outputDirName: string | undefined,
): Promise<string> {
    if (outputDirName == undefined) {
        await requestPermissions('.')
        return path.resolve(Deno.cwd())
    }

    await requestPermissions(outputDirName)
    const outputDir = path.resolve(outputDirName!)

    await ensureDir(outputDir)

    return outputDir
}

async function writeFile(
    outputPath: string,
    filePath: string,
    content: string | ArrayBufferLike,
    options: generator.FileOptions | undefined,
) {
    const output = path.join(outputPath, filePath)
    await tryMkdirs(path.dirname(output))

    const writeOptions: Deno.WriteFileOptions = {
        mode: options?.executable ? 0o744 : undefined,
    }

    if (content instanceof ArrayBuffer) {
        const data = new Uint8Array(content)
        await Deno.writeFile(output, data, writeOptions)
    } else {
        await Deno.writeTextFile(
            output,
            content as string,
            writeOptions,
        )
    }
}

async function tryMkdirs(path: string) {
    try {
        await Deno.mkdir(path, {
            recursive: true,
        })
    } catch (error) {
        if (!(error instanceof Deno.errors.AlreadyExists)) {
            throw error
        }
    }
}

async function requestPermissions(outputDir: string) {
    const permissions: Deno.PermissionDescriptor[] = [
        {
            name: 'read',
            path: Deno.cwd(), // We need this for all operations, path.resolve requries it.
        },
        {
            name: 'read',
            path: outputDir,
        },
        {
            name: 'write',
            path: outputDir,
        },
        {
            name: 'net',
            host: 'meta.fabricmc.net',
        },
        {
            name: 'net',
            host: 'maven.fabricmc.net',
        },
    ]

    for (const permission of permissions) {
        const status = await Deno.permissions.request(permission)

        if (status.state != 'granted') {
            console.error(error('Permission not granted'))
            Deno.exit(1)
        }
    }
}
