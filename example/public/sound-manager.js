/** @type {0} */
const SOURCE = 0;
/** @type {1} */
const FILE = 1;
/** @type {2} */
const NUMS = 2;
/** @type {3} */
const LANG = 3;
/** @type {"_"} */
export const NO_LANG = "_"

/** @type {0} */
export const RUNNING = 0
/** @type {1} */
export const CLOSING = 1
/** @type {2} */
export const DISPOSED = 2

class DisposableEventTarget extends EventTarget {
  constructor() {
      super();
      // Initialize a list to keep track of event listeners
      this.listeners = [];
  }

  // Override addEventListener to track listeners
  addEventListener(type, callback, options) {
      super.addEventListener(type, callback, options);
      // Add the listener details to the tracking list
      this.listeners.push({ type, callback, options });
  }

  // Implement the dispose method to remove all event listeners
  dispose() {
      // Iterate over all tracked listeners and remove them
      for (const { type, callback, options } of this.listeners) {
          super.removeEventListener(type, callback, options);
      }
      // Clear the list after removing all listeners
      this.listeners = [];
  }
}

export class SoundManager extends DisposableEventTarget {
  /**
   * @param {Record<string, ReadonlyArray<readonly [string, string, number, string]>} atlas - The sound atlas to use.
   * @param {AudioContext} [context] - The audio context to use. Default new AudioContext({ sampleRate: 48000 })
   * @param {string} [packageName] - The name of the package to use initially. Default "none"
   * @param {string} [path] - The path to the sound files. Default "./encoded/"
   * @param {string} [language] - The initial language to use. Default NO_LANG
   * @param {".webm" | ".mp4"} [extension] - The extension of the sound files to use. Default ".webm"
   */
  constructor(
    atlas = {},
    context = new AudioContext({ sampleRate: 48000 }),
    packageName = "none",
    path = "./encoded/",
    language = NO_LANG,
    extension = ".webm"
  ) {
    super()
    /** @type {AudioContext} */
    this.context = context;
    /** @type {Record<string, ReadonlyArray<readonly [string, string, number, string]>} atlas */
    this.atlas = atlas;
    /** @type {string} */
    this.cpn = packageName;
    /** @type {string} */
    this.path = path;
    /** @type {string} */
    this.language = language;
    /** @type {".webm" | ".mp4"} */
    this.extension = extension;
    /** @type {ReadonlyArray<readonly [string, string, number, string]>} */
    this.pkg = atlas[packageName] ?? [];

    /**
     * In which order to load the sounds.
     * When using the load methods.
     * @type {readonly string[]}
     */
    this.priorities = [];

    /** @type {Map<string, Promise<AudioBuffer>} */
    this.promises = new Map();
    /** @type {Map<string, AudioBuffer} */
    this.buffers = new Map();
    /** @type {typeof RUNNING | typeof DISPOSED} */
    this.state = RUNNING
  }

  /**
   * Loads an atlas json file from a url.
   * @param {string} url
   * @returns {Promise<void>}
   * @example
   * manager.loadAtlas("https://example.com/sounds.atlas.json")
   * manager.loadPackages()
   * // Will load the sounds specified in "https://example.com/sounds.atlas.json"
   **/
  async loadAtlas(url = `${this.path}.atlas.json`) {
    if (this.state !== RUNNING) {
      return
    }
    return fetch(url).then(response => response.json()).then(atlas => {
      this.atlas = atlas;
      this.dispatchEvent(new Event('atlasloaded'));
    })
  }

  /**
   * Updates the atlas with a new one.
   * Probably its best to use the reloadWithAtlas method instead.
   * Since this method does not dispose of the current assets.
   * But if you know you want to keep the assets, you can use this method.
   *
   * @param {Record<string, ReadonlyArray<readonly [string, string, number, string]>} atlas
   * @returns {void}
   * @example
   * manager.setAtlas({
   *  "main": [["a", "24kb.2ch.12372919168763747631", 12372919168763747631, "en"], ...],
   *  "localised": [["a", "24kb.2ch.12372919168763747631", 12372919168763747631, "en"], ...]
   * })
   * manager.loadPackages()
   */
  setAtlas(atlas) {
    if (this.state !== RUNNING) {
      return
    }
    this.atlas = atlas;
    this.dispatchEvent(new Event('atlasloaded'));
  }

  /**
   * @param {string} language
   * @return {boolean} - True if the language was changed, false otherwise.
   * @example
   * manager.addEventListener("languagechanged", () => {
   *   console.log("Language changed")
   * })
   * const changed = manager.setLanguage("en")
   * // changed === true
   * // "Language changed" will be logged
   * manager.setLanguage("en") // false
   */
  setLanguage(language) {
    if (this.state !== RUNNING) {
      return false
    }
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
   * @example
   * manager.addEventListener("packagechanged", () => {
   *  console.log("Package changed")
   * })
   * const changed = manager.setPackageByName("main")
   * // changed === true
   * // "Package changed" will be logged
   * manager.setPackageByName("main") // false
   */
  setPackageByName(name) {
    if (this.state !== RUNNING || name === this.cpn) {
      return false
    }
    const pkg = this.atlas[name];
    if (pkg === undefined) {
      console.debug("Package not found", name)
      return false;
    }
    this.cpn = name;
    this.pkg = pkg;
    this.dispatchEvent(new Event('packagechanged'));
    return true
  }

  /**
   * Get the specified package or the current by default.
   * @param {string} name
   * @returns {ReadonlyArray<readonly [string, string, number, string]>}
   * @example
   * const pkg = manager.getPackage()
   * // pkg === [["a", "24kb.2ch.12372919168763747631", 12372919168763747631, "en"], ...]
   */
  getPackage(name = this.cpn) {
    // needed while closing
    if (this.state === DISPOSED) {
      return []
    }
    return this.atlas[name] ?? [];
  }

  /**
   * Get the specified packages or all of them by default.
   * @param {string[]} names
   * @returns {ReadonlyArray<ReadonlyArray<readonly [string, string, number, string]>>}
   * @example
   * const packages = manager.getPackages()
   * // packages === [[["a", "24kb.2ch.12372919168763747631", 12372919168763747631, "en"], ...], ...]
   * @example
   * const packages = manager.getPackages(["main", "localised"])
   * // packages === [[["a", "24kb.2ch.12372919168763747631", 12372919168763747631, "en"], ...], ...]
   */
  getPackages(names = undefined) {
    // this is needed while closing
    if (this.state === DISPOSED) {
      return []
    }
    return this.getPackageNames(names).map(name => this.getPackage(name));
  }

  /**
   * Get the names of all the packages.
   * @param {string[] | undefined} names - The select names of the packages to use.
   * @returns {string[]}
   * @example
   * const names = manager.getPackageNames()
   * // names === ["main", "localised"]
   */
  getPackageNames(names = undefined) {
    // this is needed while closing
    if (this.state === DISPOSED) {
      return []
    }
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
   * @example
   * const names = manager.names()
   * // names === ["a", "b", "c"]
   */
  sourceNames(packageName = this.cpn, languages = [this.language]) {
    if (this.state !== RUNNING) {
      return []
    }
    const pkg = this.getPackage(packageName)
    return pkg
      .map(arr => arr[SOURCE])
      .filter((_, index) => languages.includes(pkg[index][LANG]))
  }

  /**
   * Get a unique list of languages available in the selected package (default current package).
   * @returns {string[]}
   * @example
   * const languages = manager.languages()
   * // languages === ["en", "fr", "es"]
   * @example
   * const languages = manager.languages("another_package")
   * // languages === ["en", "sv"]
   */
  languages(name = this.cpn) {
    if (this.state !== RUNNING) {
      return []
    }
    return [
        ...new Set(this.getPackage(name).map(
          items => items[LANG])
        )
    ];
  }

  /**
   * Set the path to the sound files.
   * @param {string} path
   * @returns {void}
   * @example
   * manager.setLoadPath("https://example.com/sounds/")
   * manager.loadPackageName()
   * // Will load the sounds from "https://example.com/sounds/" base, instead of the default "./encoded/".
   */
  setLoadPath(path) {
    if (this.state !== RUNNING) {
      return
    }
    this.path = path;
    this.dispatchEvent(new Event('loadpathchange'));
  }

  /**
   * Set the load priorities.
   * @param {string[]} sources
   * @returns {void}
   * @example
   * manager.setPriorityList(["a", "b", "c"])
   * manager.loadPackageName()
   * // Will load "a" first, then "b", then "c"
   * // then the rest of the sounds in the package
   */
  setPriorityList(sources) {
    this.priorities = sources
  }

  /**
   * Tries to get the buffer from the current package, if it fails, it will try to get it from all packages
   * If that fails, it will return null.
   * Null is also a playable sound, it will just not play anything.
   * @param {string} name
   * @returns {Promise<AudioBuffer | null>}
   * @example
   * const buffer = await manager.requestBufferAsync("main_music")
   * // buffer === AudioBuffer
   */
  async requestBufferAsync(name) {
    if (this.state !== RUNNING) {
      return null
    }
    const file = this.getFileName(name);
    return file === undefined ? null : this.loadFile(file);
  }

  /**
   * Get a previously loaded sound buffer.
   * Otherwise, it will an empty buffer of the correct duration.
   * Unless there is an issue with the name provided, in which case it will return null.
   * Null is also assignable to an AudioBufferSourceNode, it will just not play anything.
   * If the sound is not loaded, it will try to load it.
   * After it has been loaded, the buffer contents will be updated in place.
   * So the sound will continue playing from whatever point it had gotten to.
   * Meaning you can schedule your sounds and expect them to play in sync.
   * Even if they had not been preloaded, there might just be some silence in the beginning
   * until the network request is finished.
   * @param {string} name - the name of the sound to get.
   * @returns {AudioBuffer | null}
   * @example
   * const buffer = manager.requestBufferSync("main_music")
   * // buffer === AudioBuffer
   */
  requestBufferSync(name) {
    if (this.state !== RUNNING) {
      return null
    }
    const item = this.findItemBySourceName(name);
    if (item === undefined) {
      return null;
    }
    const file = item[FILE];
    const buffer = this.buffers.get(file);
    if (buffer !== undefined) {
      return buffer;
    }
    const nc = this.numChannels(file);
    if (nc === undefined) {
      return null;
    }
    const silence = this.context.createBuffer(
      nc,
      item[NUMS],
      this.context.sampleRate
    );
    this.buffers.set(file, silence);
    this.loadItem(item)
    return silence
  }

  /**
   * Get the number of channels of a sound.
   * @param {string} file - the sound file to get.
   * @return {number | undefined} - The number of channels of the sound or undefined if it fails.
   * @example
   * const numChannels = manager.numChannels("24kb.2ch.12372919168763747631")
   * // numChannels === 2
   */
  numChannels(file) {
    try {
      return Number(file.split(".")[1].replace("ch", ""))
    } catch (_) {
      return undefined
    }
  }

  /**
   * Get the number of samples of a sound.
   * @param {string} file - the sound file to get.
   * @return {number | undefined} - The number of samples of the sound or undefined if it fails.
   * @example
   * const numSamples = manager.numSamples("24kb.2ch.12372919168763747631")
   * // numSamples === 12372919168763747631
   */
  numSamples(file) {
    return this.findItemByFileName(file)[NUMS];
  }

  /**
   * Will return the sound item if it exists in the package with the current language or if it has no language assigned.
   * If it does not exist, it will return undefined.
   * We put arr[LANG] === NO_LANG first in the condition because we want to allow sounds with no language to be played.
   * And most packages are not localized, so we want to allow them to be played.
   * @param {string} sourceName - the name of the sound to get.
   * @param {string | undefined} - The name of package to search in.
   * @param {string | undefined} - The name of package to search in.
   * @returns {readonly [string, string, number, string] | undefined}
   * @example
   * const item = manager.findItemBySourceName("main_music")
   * // item === ["main_music", "24kb.2ch.12372919168763747631", 12372919168763747631, "en"]
   */
  findItemBySourceName(sourceName, packageName = this.cpn, language = this.language) {
    if (this.state !== RUNNING) {
      return
    }
    return this.getPackage(packageName).find(item =>
      item[SOURCE] === sourceName && (item[LANG] === NO_LANG || item[LANG] === language)
    );
  }

  /**
   * Will return the sound item if it exists in the package with the current language or if it has no language assigned.
   * If it does not exist, it will return undefined.
   * We put `arr[LANG] === NO_LANG` first in the condition because we want to allow sounds with no language to be played.
   * And most packages are not localized, so we want to allow them to be played.
   * @param {string} sourceName - the name of the sound to get.
   * @param {string | undefined} packageName - The name of package to search in.
   * @returns {readonly [string, string, number, string] | undefined}
   * @example
   * const item = manager.findItemByFilename("24kb.2ch.12372919168763747631")
   * // item === ["main_music", "24kb.2ch.12372919168763747631", 12372919168763747631, "en"]
   */
  findItemByFileName(fileName, packageName) {
    if (this.state !== RUNNING) {
      return
    }
    const pkg = this.getPackage(packageName);
    return pkg.find(item => item[FILE] === fileName);
  }

  /**
   * Get the output file name of a sound from the original file name.
   * @param {string} sourceName - the name of the sound to get.
   * @param {string} packageName - the name of the package to use.
   * @param {string} language - the language to use.
   * @returns {string | undefined} file - the output file.
   * @example
   * const file = manager.getFileName("main_music")
   * // file === "24kb.2ch.12372919168763747631"
   */
  getFileName(sourceName, packageName = this.cpn, language = this.language) {
    if (this.state !== RUNNING) {
      return
    }
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
   * Load a sound file based on the filename.
   * @param {string} file - the sound file to load.
   * @returns {Promise<AudioBuffer | null>}
   * @example
   * const audioBuffer = await manager.loadFile("24kb.2ch.12372919168763747631")
   */
  async loadFile(file) {
    if (this.state !== RUNNING) {
      return null
    }
    const promise = this.promises.get(file)
    if (promise !== undefined) {
      return promise
    }
    const item = this.findItemByFileName(file);
    if (item === undefined) {
      this.dispatchEvent(new CustomEvent("soundloaderror", { detail: { file } }))
      return null;
    }
    return this.loadItem(item);
  }

  /**
   * Load a sound file given the atlas item.
   * @param {[string, string, number, string]} item - the atlas item to load.
   * @returns {Promise<AudioBuffer | null>}
   */
  async loadItem(item) {
    if (this.state !== RUNNING) {
      return null
    }
    const file = item[FILE];
    const promise = this.promises.get(file) ?? fetch(this.path + file + this.extension)
      .then(response => response.arrayBuffer())
      .then(buffer => this.context.decodeAudioData(buffer))
      .then(decoded => {
        const nums = item[NUMS];
        const preexisting = this.buffers.get(file)
        const target = preexisting ?? this.context.createBuffer(
          decoded.numberOfChannels,
          nums,
          this.context.sampleRate
        )
        this.buffers.set(file, target);
        fill(target, decoded);
        return target;
      })
      .then(_ => {
        this.dispatchEvent(new CustomEvent("soundloaded", { detail: { file } }))
      })
      .catch(_ => {
        console.warn("Failed to load sound", file, _)
        return this.buffers.get(file) ?? null;
      });
    this.promises.set(file, promise);
    return promise;
  }

  /**
   * Load all the items.
   * With whichever load priority has been specified.
   * @param {ReadonlyArray<[string, string, number, string]>} items - the atlas items to load.
   * @returns {Promise<AudioBuffer[]>}
   * @example
   * const audioBuffers = await manager.loadItems([["music", "<fname>", 1, "en"], ["effect", "<fname>", 2, "en"]])
   */
  async loadItems(items) {
    const sorted = this.priorities.length > 0 ? sortItems([...items], this.priorities) : items;
    return Promise.all(sorted.map(item => this.loadItem(item)));
  }

  /**
   * Load the the specified sounds by original source name.
   * @param {string[]} sources
   * @returns {Promise<AudioBuffer[]>}
   * @example
   * const audioBuffers = await manager.loadSources(["a", "b", "c"])
   */
  async loadSources(sources) {
    const items = sources.map(source => this.findItemBySourceName(source)).filter(item => item !== undefined);
    return this.loadItems(items);
  }

  /**
   * Load all sounds in a specified package.
   * Including all languages.
   * @returns {Promise<AudioBuffer[]>}
   */
  async loadPackageName(name = this.cpn) {
    if (this.state !== RUNNING) {
      return []
    }
    return this.loadItems(this.getPackage(name));
  }

  /**
   * Load all sounds in the specified packages.
   * Including all languages.
   * @parmam {string[]} names - The names of the packages to load.
   * @returns {Promise<AudioBuffer[]>}
   */
  async loadPackageNames(names = undefined) {
    if (this.state !== RUNNING) {
      return []
    }
    return Promise.all(this.getPackageNames(names).map(name => this.loadPackageName(name)));
  }

  /**
   * Load all packages,
   * all languages, everything.
   * @returns {Promise<AudioBuffer[]>}
   */
  async loadEverything() {
    if (this.state !== RUNNING) {
      return []
    }
    return this.loadPackageNames()
  }

  /**
   * Load all sounds in the specified language.
   * For the specified packages (default `[current]`) package.
   * @param {string} language
   * @param {string} packageName
   * @return {Promise<AudioBuffer[]>}
   * @example
   * await manager.loadLanguage("en", ["package1", "package2"])
   */
  async loadLanguage(language = this.language, packageNames = [this.cpn]) {
    if (this.state !== RUNNING) {
      return []
    }
    return Promise.all(
      this.getPackages(packageNames)
        .map(pack => pack
          .filter(item => item[LANG] === language)
          .map(item => this.loadItem(item))
        )
    );
  }

  /**
   * Load all sounds in the specified languages (default `[current_language]`)
   * For the specified packages (default `[current_package]`) package.
   * @param {string[]} languages
   * @param {string[]} packageNames
   */
  async loadLanguages(languages = [this.language], packageNames = [this.cpn]) {
    if (this.state !== RUNNING) {
      return []
    }
    return Promise.all(languages.map(language => this.loadLanguage(language, packageNames)));
  }

  /**
   * Loads the sounds in the priority list.
   * @returns {Promise<AudioBuffer[]>}
   * @example
   * const manager = new SoundManager()
   * manager.setPriorityList(["a", "b", "c"])
   * manager.loadPriorityList()
   */
  async loadPriorityList() {
    return this.loadSources(this.priorities)
  }

  /**
   * Dispose of a single item.
   * @param {[string, string, number, string]} item
   * @returns {void}
   */
  async disposeItem(item) {
    if (this.state === DISPOSED) {
      return
    }
    const file = item[FILE];
    this.buffers.delete(file);
    const promise = this.promises.get(file);
    if (promise !== undefined) {
      this.promises.delete(file);
      return promise.finally(_ => {
        this.buffers.delete(file)
        this.promises.delete(file)
      });
    }
  }

  /**
   * Dispose of all the sounds in the specified package.
   * @param {string} name, or undefined to dispose of the current package.
   * @returns {Promise<void>}
   */
  async disposePackage(name = this.cpn) {
    if (this.state === DISPOSED) {
      return
    }
    const items = this.getPackage(name);
    return Promise.all(items.map(item => this.disposeItem(item)));
  }

  /**
   * Dispose of all the sounds in the specified packages.
   * @param {string[] | undefined} names, or undefined to dispose of all packages.
   * @returns {Promise<void>}
   */
  async disposePackages(names = undefined) {
    if (this.state === DISPOSED) {
      return
    }
    const packs = this.getPackages(names);
    return Promise.all(packs.map(pack => pack.map(item => this.disposeItem(item))));
  }

  /**
   * Dispose of all the sounds With the specified language.
   * In the specified packages.
   * @param {string} language
   * @param {string[]} packageNames
   */
  async disposeLanguage(language = this.language, packageNames = undefined) {
    if (this.state === DISPOSED) {
      return
    }
    const packs = this.getPackages(packageNames);
    /** @type {Promise<void>} */
    const promises = [];
    for (const pack of packs) {
      for (const item of pack) {
        if (item[LANG] === language) {
          promises.push(this.disposeItem(item))
        }
      }
    }
    return Promise.all(promises);
  }

  /**
   * Dispose the sound manager
   * @returns {Promise<void>}
   */
  async dispose(disposeListeners = true) {
    if (this.state === DISPOSED) {
      return
    }
    // remove all event listeners
    if (disposeListeners) {
      super.dispose()
    }
    this.state = CLOSING
    return this.disposePackages().catch(() => {
      console.warn("Failed to dispose")
    }).finally(() => {
      this.state = DISPOSED
    })
  }

  /**
   * Reload the sound manager.
   * This will dispose of the current assets.
   * Can be used if you want to free up memory.
   * And use the sound manager with a new atlas.
   * For example if you have a single page application
   * Where you are loading a new sound atlas for each page.
   * And dont want to bloat the users memory with unused assets.
   * @param {boolean | undefined} disposeListeners - If true, it will dispose of all the listeners attached.
   * @returns {Promise<void>}
   */
  async reload(disposeListeners = false) {
    return this.dispose(disposeListeners).finally(() => {
      this.state = RUNNING
      this.dispatchEvent(new Event('reloaded'));
    })
  }

  /**
   * Reload the sound manager with a new atlas.
   * This will dispose of the current assets.
   * Can be used if you want to free up memory.
   * And use the sound manager with a new atlas.
   * For example if you have a single page application
   * Where you are loading a new sound atlas for each page.
   * And dont want to bloat the users memory with unused assets.
   * @param {Record<string, ReadonlyArray<readonly [string, string, number, string]>} atlas
   * @param {boolean | undefined} disposeListeners - If true, it will dispose of all the listeners attached.
   * @returns {Promise<void>}
   */
  async reloadWithAtlas(atlas = this.atlas, disposeListeners = false) {
    return this.reload(disposeListeners).then(() => {
      this.setAtlas(atlas)
    })
  }

  // todo: add a method to dispose of a single sound
  // todo: add a method to dispose of a single package
  // todo: add a method to dispose of a single language
}

/**
 * Fill a target buffer with the contents of a source buffer.
 * Works also for legacy browsers that do not support copyToChannel.
 *
 * @param {AudioBuffer} target
 * @param {AudioBuffer} source
 * @returns {void}
 */
function fill(target, source) {
  const nc = Math.min(target.numberOfChannels, source.numberOfChannels)
  if (target.copyToChannel === undefined) {
    const ns = Math.min(target.length, source.length)
    for (let i = 0; i < nc; i++) {
      const tch = target.getChannelData(i);
      const sch = source.getChannelData(i);
      for (let j = 0; j < ns; j++) {
        tch[j] = sch[j]
      }
    }
  } else {
    for (let i = 0; i < nc; i++) {
      target.copyToChannel(source.getChannelData(i), i, 0);
    }
  }
}

/**
 * Sort the items by the priorities.
 * @param {[string, string, number, string][]} items
 * @param {string[]} priorities
 */
function sortItems(items, priorities) {
  const priorityIndex = new Map(priorities.map((source, index) => [source, index]));
  return items.sort((a, b) => {
      const aIndex = priorityIndex.has(a[SOURCE]) ? priorityIndex.get(a[SOURCE]) : Infinity;
      const bIndex = priorityIndex.has(b[SOURCE]) ? priorityIndex.get(b[SOURCE]) : Infinity;
      return aIndex - bIndex;
  });
}
