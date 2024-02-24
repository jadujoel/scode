
main();
/**
 * @param {SoundManager} manager
 * @param {Function} onclick
 * @returns {HTMLButtonElement}
 */
function createButton(name, onclick) {
  const button = document.createElement('button');
  button.textContent = name;
  document.body.appendChild(button);
  button.addEventListener('click', onclick);
  return button;
}

class SoundManager {
  /**
   * @param {Record<string, ReadonlyArray<readonly [string, string, number, string]>} info
   * @param {AudioContext} [context]
   */
  constructor(info, context = new AudioContext()) {
    this.context = context;
    this.info = info;
    this.language = "none";
    this.pkg = [];
    this.buffers = [];
    this.packageName = "";
    this.extension = ".webm";
    /** @type {Map<string, Promise<AudioBuffer>} */
    this.urls = new Map();
    this.loadPath = "/encoded/";
  }
  /**
   * @param {string} language
   */
  setLanguage(language) {
    this.language = language;
  }
  /**
   * @param {string} name
   */
  setPackage(name) {
    this.packageName = name;
    this.pkg = this.info[name] ?? this.pkg;
  }

  /**
   * @returns {ReadonlyArray<readonly [string, string, number, string]>}
   */
  package() {
    return this.info[this.packageName] ?? [];
  }

  /**
   * @returns {string[]}
   */
  names() {
    const pkg = this.package();
    if (!pkg) return [];
    return pkg.map(arr => arr[0]);
    return (pkg.map(arr => arr[0]) ?? [])
      .filter((_, index) => {
        const lang = pkg[index][3];
        lang === this.language || lang === undefined;
      })
  }

  /**
   * @returns {string[]}
   */
  languages() {
    return [...new Set(this.package().map(arr => arr[3]).filter(x => x !== undefined))];
  }

  /**
   * @returns {string[]}
   */
  packages() {
    return Object.keys(this.info);
  }

  /**
   * @param {string} path
   */
  setLoadPath(path) {
    this.loadPath = path;
  }

  /**
   * @param {string} name
   */
  async getBuffer(name) {
    let pkg = this.package();
    const url = pkg?.find(arr => arr[0] === name && (arr[3] === undefined || arr[3] === this.language))[1];
    if (url) return await this.load(url);
    // if we couldnt find the sound in the current package, we will search in all packages
    for (const pkg of Object.values(this.info)) {
      if (pkg === this.pkg) continue;
      const url = pkg.find(arr => arr[0] === name && (arr[3] === undefined || arr[3] === this.language))[1];
      if (url) return await this.load(url);
    }
    return null
  }

  /**
   * @param {string} file
   * @returns {Promise<AudioBuffer>}
   */
  async load(file) {
    let promise = this.urls.get(file);
    if (promise) return promise;
    promise = fetch(this.loadPath + file + this.extension)
      .then(response => response.arrayBuffer())
      .then(buffer => this.context.decodeAudioData(buffer))
      .catch(_ => null);
    this.urls.set(file, promise);
    return promise;
  }
}

async function main() {
  const response = await fetch('/encoded/.info.json');
  /** @type {Record<string, ReadonlyArray<readonly [string, string, number, string]>} */
  const data = await response.json();
  const context = new AudioContext();
  const manager = new SoundManager(data, context);
  console.log(manager.info)
  manager.setLanguage("en");
  manager.setPackage("videopoker");
  // console.log("package", manager.package())
  // console.log("packages", manager.packages())
  // console.log("languages", manager.languages())
  // console.log("names", manager.names())
  // console.log("package name", manager.packageName)

  /** @type {HTMLButtonElement[]} */
  let playButtons = []
  function clearPlayButtons() {
    playButtons.forEach(button => {
      button?.remove()
    });
    playButtons = [];
  }
  /** @type {HTMLButtonElement[]} */
  let languageButtons = [];
  function clearLanguageButtons() {
    languageButtons.forEach(button => {
      button?.remove()
    });
    languageButtons = [];
  }

  for (const packageName of manager.packages()) {
    createButton(packageName, () => {
      manager.setPackage(packageName);
      clearLanguageButtons();
      languageButtons = createLanguageButtons(manager);
      clearPlayButtons();
      playButtons = createPlayButtons(manager);
      console.log(manager.languages())
    })
  }
  addDivider();
  for (const language of manager.languages()) {
    createButton(language, () => {
      manager.setLanguage(language);
      buttons.forEach(button => button.remove());
      buttons = createPlayButtons(manager);
    })
  }
  addDivider();
}

function createLanguageButtons(manager) {
  return manager.languages().map(language => {
    return createButton(language, () => {
      manager.setLanguage(language);
    })
  })
}

function addDivider() {
  document.body.appendChild(document.createElement('hr'));
}

/**
 * @param {SoundManager} manager
 * @returns {HTMLButtonElement[]}
 */
function createPlayButtons(manager) {
  return manager.names().map(name => {
    return createButton(name, () => {
      const source = manager.context.createBufferSource();
      manager.getBuffer(name).then(buffer => {
        source.buffer = buffer;
        source.connect(manager.context.destination);
        source.start();
      });
    })
  })
}
