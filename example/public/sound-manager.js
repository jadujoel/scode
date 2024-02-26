/** @type {0} */
const NAME = 0;
/** @type {1} */
const FILE = 1;
/** @type {2} */
const NUMS = 2;
/** @type {3} */
const LANG = 3;
/** @type {"_"} */
export const NO_LANG = "_"

export class SoundManager extends EventTarget {
  /**
   * @param {Record<string, ReadonlyArray<readonly [string, string, number, string]>} atlas - The sound atlas to use.
   * @param {AudioContext} [context] - The audio context to use.
   * @param {string} [name] - The name of the package to use.
   * @param {string} [path] - The path to the sound files.
   * @param {string} [language] - The initial language to use.
   * @param {".webm" | ".mp4"} [extension] - The extension of the sound files to use.
   */
  constructor(
    atlas = [],
    context = new AudioContext({ sampleRate: 48000 }),
    name = NO_LANG,
    path = "./encoded/",
    language = NO_LANG,
    extension = ".webm",
  ) {
    super()
    this.context = context;
    this.atlas = atlas;
    this.package = name;
    this.path = path;
    this.language = language;
    this.extension = extension;
    this.pkg = [];
    /** @type {Map<string, Promise<AudioBuffer>} */
    this.promises = new Map();
    /** @type {Map<string, AudioBuffer} */
    this.buffers = new Map();
  }

  /**
   * @param {string} url
   **/
  async loadAtlas(url = `${this.path}.atlas.json`) {
    this.atlas = await fetch(url).then(response => response.json());
    this.dispatchEvent(new Event('atlasloaded'));
  }

  /**
   * @param {string} language
   * @return {boolean} - True if the language was changed, false otherwise.
   */
  setLanguage(language) {
    if (this.language === language) {
      return false
    }
    if (!this.languages().includes(language)) {
      return false
    }
    this.language = language;
    this.dispatchEvent(new Event('languagechanged'));
    return true
  }
  /**
   * Returns true if the package exists, false otherwise.
   * @param {string} name - The name of the package to use.
   * @returns {boolean} - True if the package exists, false otherwise.
   */
  setPackage(name) {
    if (name === this.package) {
      return false
    }
    const pkg = this.atlas[name];
    if (pkg === undefined) {
      console.debug("Package not found", name)
      return false;
    }
    this.package = name;
    this.pkg = pkg;
    this.dispatchEvent(new Event('packagechanged'));
    return true
  }

  /**
   * Get the specified package or the current by default.
   * @param {string} name
   * @returns {ReadonlyArray<readonly [string, string, number, string]>}
   */
  getPackage(name = this.package) {
    return this.atlas[name] ?? [];
  }

  /**
   * Get the names of all the packages.
   * @param {string[]} names - The select names of the packages to use.
  * @returns {string[]}
  */
  getPackages(names) {
    if (names !== undefined) {
      return names.filter(name => this.atlas[name] !== undefined)
    }
    return Object.keys(this.atlas);
  }

  /**
   * Get the names of all the sounds in the current package.
   * With the current language, or no language.
   * @param {string} packageName - The name of the package to use.
   * @param {string[]} languages - The languages to use, default current language.
   * @returns {string[]}
   */
  names(packageName = this.package, languages = [this.language]) {
    const pkg = this.getPackage(packageName)
    return pkg
      .map(arr => arr[NAME])
      .filter((_, index) => languages.includes(pkg[index][LANG]))
  }

  /**
   * Get the languages available in current package.
   * @returns {string[]}
   */
  languages(name = this.package) {
    return [
        ...new Set(this.getPackage(name).map(
          items => items[LANG])
        )
    ];
  }

  /**
   * Set the path to the sound files.
   * @param {string} path
   */
  setLoadPath(path) {
    this.path = path;
    this.dispatchEvent(new Event('loadpathchange'));
  }

  /**
   * Tries to get the buffer from the current package, if it fails, it will try to get it from all packages
   * If that fails, it will return null.
   * Null is also a playable sound, it will just not play anything.
   * @param {string} name
   * @returns {Promise<AudioBuffer | null>}
   */
  async requestBufferAsync(name) {
    const file = this.getFileName(name);
    if (file !== undefined) {
      return this.load(file);
    }
    return null
  }

  /**
   * Get a previously loaded sound buffer.
   * Otherwise, it will return null.
   * If the sound is not loaded, it will try to load it.
   * So that it is available in the future.
   * @param {string} name - the name of the sound to get.
   * @returns {AudioBuffer | null}
   */
  requestBufferSync(name) {
    const file = this.getFileName(name);
    const buffer = this.buffers.get(file) ?? null;
    if (buffer === null) {
      this.requestBufferAsync(name).catch()
    }
    return buffer
  }

  /**
   * Will return the sound item if it exists in the package with the current language or if it has no language assigned.
   * If it does not exist, it will return undefined.
   * We put arr[LANG] === NO_LANG first in the condition because we want to allow sounds with no language to be played.
   * And most packages are not localized, so we want to allow them to be played.
   * @param {string} sourceName - the name of the sound to get.
   * @param {string | undefined} - The name of package to search in.
   * @returns {readonly [string, string, number, string] | undefined}
   */
  findItemBySourceName(sourceName, pkgName, language = this.language) {
    const pkg = this.getPackage(pkgName);
    return pkg.find(item =>
      item[NAME] === sourceName
      && (item[LANG] === NO_LANG || item[LANG] === language)
    );
  }

  /**
   * Will return the sound item if it exists in the package with the current language or if it has no language assigned.
   * If it does not exist, it will return undefined.
   * We put arr[LANG] === NO_LANG first in the condition because we want to allow sounds with no language to be played.
   * And most packages are not localized, so we want to allow them to be played.
   * @param {string} sourceName - the name of the sound to get.
   * @param {string | undefined} - The name of package to search in.
   * @returns {readonly [string, string, number, string] | undefined}
   */
  findItemByFileName(fileName, pkgName, language = this.language) {
    const pkg = this.getPackage(pkgName);
    return pkg.find(item => item[FILE] === fileName);
  }

  /**
   * Get the output file name of a sound from the original file name.
   * @param {string} sourceName - the name of the sound to get.
   * @param {string} packageName - the name of the package to use.
   * @param {string} language - the language to use.
   * @returns {string | undefined} file - the output file.
   */
  getFileName(sourceName, packageName = this.package, language = this.language) {
    // Search in the selected package
    const file = this.findItemBySourceName(sourceName, packageName, language)[FILE]
    if (file !== undefined) {
      return file;
    }

    // Search in the other packages
    for (const packageName of Object.values(this.atlas)) {
      const file = this.findItemBySourceName(sourceName, packageName, language)[FILE];
      if (file) {
        return file;
      }
    }

    // failed to find the associated file
    return undefined;
  }

  /**
   * Load a sound file.
   * @param {string} file - the sound file to load.
   * @returns {Promise<AudioBuffer>}
   */
  async load(file) {
    let promise = this.promises.get(file);
    if (promise !== undefined) {
        return promise;
    }
    const item = this.findItemByFileName(file)
    if (item === undefined) {
      console.error("Failed to find item for", file)
      this.dispatchEvent(new CustomEvent("soundloaderror", { detail: { file } }))
      return null;
    }
    promise = fetch(this.path + file + this.extension)
      .then(response => response.arrayBuffer())
      .then(buffer => this.context.decodeAudioData(buffer))
      .then(decoded => {
        const nums = item[NUMS];
        if (nums === decoded.length) {
          this.buffers.set(file, decoded);
          return decoded;
        }
        // sometimes firefox is a bit off with the number of samples
        // so we will just create a new buffer with the correct number of samples
        // we dont want any drifting
        const edited = this.context.createBuffer(decoded.numberOfChannels, nums, this.context.sampleRate);
        for (let i = 0; i < decoded.numberOfChannels; i++) {
          // copyToChannel is not supported in some browsers
          try {
            edited.copyToChannel(decoded.getChannelData(i), i);
          } catch (_) {
            for (let j = 0; j < nums; j++) {
              edited.getChannelData(i)[j] = decoded.getChannelData(i)[j];
            }
          }
        }
        this.buffers.set(file, edited);
        return edited;
      })
      .catch(_ => {
        console.error("Failed to load sound", file, _)
        this.buffers.set(file, null);
        return null;
      });
    this.promises.set(file, promise);
    promise.then(_ => {
      this.dispatchEvent(new CustomEvent("soundloaded", { detail: { file } }))
    })
    return promise;
  }

  /**
   * Load all sounds in a specified package.
   * @returns {Promise<AudioBuffer[]>}
   */
  async loadPackage(name = this.package) {
    const promises = this.getPackage(name).map(arr => this.load(arr[FILE]));
    return Promise.all(promises);
  }

  /**
   * Load all sounds in the specified packages.
   * @parmam {string[]} names - The names of the packages to load.
   * @returns {Promise<AudioBuffer[]>}
   */
  async loadPackages(names = undefined) {
    const promises = this.getPackages(names).map(name => this.loadPackage(name));
    return Promise.all(promises);
  }

  /**
   * Load all sounds in the specified language.
   * For the current package.
   * @param {string} language
   * @return {Promise<AudioBuffer[]>}
   */
  async loadLanguage(language = this.language) {
    const promises = this.getPackage()
      .filter(arr => arr[LANG] === language)
      .map(arr => this.load(arr[FILE]));
    return Promise.all(promises);
  }
}
