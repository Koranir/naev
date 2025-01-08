use formatx::formatx;
use sdl2 as sdl;
use std::ffi::{CStr, CString};
use std::io::{Error, ErrorKind, Result};
use std::os::raw::{c_char, c_int, c_uint, c_void};

#[link(name = "naev")]
extern "C" {
    /// Main function in C
    pub fn naev_main() -> c_int;
}

mod array;
mod buffer;
mod camera;
mod damagetype;
mod env;
mod gettext;
mod linebreak;
mod log;
mod model;
mod ndata;
mod ngl;
mod nlua;
mod ntime;
mod nxml;
mod physfs;
mod physics;
mod rng;
mod shader;
mod slots;
mod start;
mod texture;
mod utils;
mod version;

use crate::gettext::gettext;

pub static APPNAME: &str = "Naev";

use std::sync::atomic::AtomicBool;
static _QUIT: AtomicBool = AtomicBool::new(false);

unsafe fn cptr_to_cstr<'a>(s: *const c_char) -> &'a str {
    CStr::from_ptr(s).to_str().unwrap()
}

pub fn naev() -> Result<()> {
    /* Load up the argv and argc for the C main. */
    let args: Vec<String> = std::env::args().collect();
    let mut cargs = vec![];
    for a in args {
        cargs.push(CString::new(a).unwrap())
    }
    let mut argv = cargs.into_iter().map(|s| s.into_raw()).collect::<Vec<_>>();
    argv.shrink_to_fit();

    /* Begin logging infrastructure. */
    log::init();

    // Workarounds
    if cfg!(target_os = "linux") {
        // Set AMD_DEBUG environment variable before initializing OpenGL to
        // workaround driver bug. TODO remove around 0.14.0 or when fixed (maybe changing
        // backend?).
        std::env::set_var("AMD_DEBUG", "nooptvariant");
    }

    /* Start up PHYSFS. */
    unsafe {
        let argv0 = CString::new(env::ENV.argv0.clone()).unwrap();
        if naevc::PHYSFS_init(argv0.as_ptr() as *const c_char) == 0 {
            let err = physfs::error_as_io_error();
            println!("{}", err);
            return Err(Error::new(ErrorKind::Other, err));
            /* TODO probably move the error handling to the "real" main, when shit hits the
                * fan. Below depends on sdl3
            SDL_ShowSimpleMessageBox( SDL_MESSAGEBOX_ERROR,
                _( "Naev Critical Error" ), buf,
                gl_screen.window );
            */
        }
        naevc::PHYSFS_permitSymbolicLinks(1);
    }

    /* Set up locales. */
    linebreak::init();
    gettext::init();

    /* Print the version */
    log::info(&version::VERSION_HUMAN);
    if cfg!(target_os = "linux") {
        match env::ENV.is_appimage {
            true => {
                info!("AppImage detected. Running from: {}", &env::ENV.appdir)
            }
            false => debug!("AppImage not detected."),
        }
    }

    /* Initialize SDL. */
    let sdlctx = match sdl::init() {
        Ok(s) => s,
        Err(e) => panic!("Unable to initialize SDL: {}", e),
    };

    let sdltime = match sdlctx.timer() {
        Ok(s) => s,
        Err(e) => panic!("Unable to initialize SDL Timer: {}", e),
    };
    let _starttime = sdltime.ticks();

    unsafe {
        naevc::threadpool_init();
        naevc::debug_sigInit();
    }

    if cfg!(unix) {
        /* Set window class and name. */
        std::env::set_var("SDL_VIDEO_X11_WMCLASS", APPNAME);
    }

    let sdlvid = match sdlctx.video() {
        Ok(s) => s,
        Err(e) => panic!("Unable to initialize SDL Video: {}", e),
    };

    unsafe {
        naevc::nxml_init(); /* We'll be parsing XML. */
        naevc::input_init(); /* input has to be initialized for config to work. */
        naevc::conf_setDefaults(); /* set the default config values. */

        /*
         * Attempts to load the data path from datapath.lua
         * At this early point in the load process, the binary path
         * is the only place likely to be checked.
         */
        naevc::conf_loadConfigPath();
    }

    /* Create the home directory if needed. */
    let cpath = unsafe { naevc::nfile_configPath() };
    unsafe {
        if naevc::nfile_dirMakeExist(cpath) != 0 {
            warn!(gettext("Unable to create config directory '{}'"), "foo");
        }
    }

    /* Set up the configuration. */
    let conf_file_path = unsafe {
        let rpath = cptr_to_cstr(cpath);
        let conf_file = CStr::from_ptr(naevc::CONF_FILE.as_ptr() as *const c_char)
            .to_str()
            .unwrap();
        format!("{}{}", rpath, conf_file)
    };

    unsafe {
        let cconf_file_path = CString::new(conf_file_path.clone()).unwrap();
        naevc::conf_loadConfig(cconf_file_path.as_ptr()); /* Lua to parse the configuration file */
        naevc::conf_parseCLI(argv.len() as c_int, argv.as_mut_ptr()); /* parse CLI arguments */

        /* Set up I/O. */
        naevc::ndata_setupWriteDir();
        naevc::log_redirect();
        naevc::ndata_setupReadDirs();
        naevc::gettext_setLanguage(naevc::conf.language); /* now that we can find translations */
        info!(gettext("Loaded configuration: {}"), conf_file_path);
        let search_path = naevc::PHYSFS_getSearchPath();
        info!(gettext("Read locations, searched in order:"));
        for p in {
            let mut out: Vec<&str> = Vec::new();
            let mut i = 0;
            loop {
                let sp = *search_path.offset(i);
                if sp.is_null() {
                    break;
                }
                let s = CStr::from_ptr(sp).to_str().unwrap();
                out.push(s);
                i += 1;
            }
            out
        } {
            info!("    {}", p);
        }
        naevc::PHYSFS_freeList(search_path as *mut c_void);

        /* Logging the cache path is noisy, noisy is good at the DEBUG level. */
        debug!(
            gettext("Cache location: {}"),
            cptr_to_cstr(naevc::nfile_cachePath())
        );
        info!(
            gettext("Write location: {}\n"),
            cptr_to_cstr(naevc::PHYSFS_getWriteDir())
        );
    }

    let _lua = nlua::NLua::new();

    unsafe {
        /* Enable FPU exceptions. */
        if naevc::conf.fpu_except != 0 {
            naevc::debug_enableFPUExcept();
        }

        if naevc::start_load() != 0 {
            let err = gettext("Failed to load start data.");
            warn!(err);
            // TODO show some simple error message
            return Err(Error::new(ErrorKind::Other, err));
        }
        info!(
            " {}\n",
            CStr::from_ptr(naevc::start_name()).to_str().unwrap()
        );

        /* Display the SDL version. */
        naevc::print_SDLversion();
        info!("");
    }

    /* Set up OpenGL. */
    let _context = ngl::init(sdlvid).unwrap();

    unsafe {
        if naevc::gl_init() != 0 {
            let err = gettext("Initializing video output failed, exiting…");
            warn!(err);
            // TODO show some simple error message
            return Err(Error::new(ErrorKind::Other, err));
        }

        //Have to set up fonts before rendering anything.
        let font_prefix = naevc::FONT_PATH_PREFIX as *const u8 as *const i8;
        let font_default_path = gettext("Cabin-SemiBold.otf,NanumBarunGothicBold.ttf,SourceCodePro-Semibold.ttf,IBMPlexSansJP-Medium.otf");
        let font_default_path_c = CString::new(font_default_path).unwrap();
        let font_small_path = gettext("Cabin-SemiBold.otf,NanumBarunGothicBold.ttf,SourceCodePro-Semibold.ttf,IBMPlexSansJP-Medium.otf" );
        let font_small_path_c = CString::new(font_small_path).unwrap();
        let font_mono_path =
            gettext("SourceCodePro-Semibold.ttf,D2CodingBold.ttf,IBMPlexSansJP-Medium.otf");
        let font_mono_path_c = CString::new(font_mono_path).unwrap();
        naevc::gl_fontInit(
            &raw mut naevc::gl_defFont,
            font_default_path_c.as_ptr(),
            naevc::conf.font_size_def as c_uint,
            font_prefix,
            0,
        );
        naevc::gl_fontInit(
            &raw mut naevc::gl_smallFont,
            font_small_path_c.as_ptr(),
            naevc::conf.font_size_small as c_uint,
            font_prefix,
            0,
        );
        naevc::gl_fontInit(
            &raw mut naevc::gl_defFontMono,
            font_mono_path_c.as_ptr(),
            naevc::conf.font_size_def as c_uint,
            font_prefix,
            0,
        );

        // Detect size changes that occurred after window creation.
        naevc::naev_resize();
    }

    // Display the initial load screen.
    unsafe {
        naevc::loadscreen_load();
        let s = CString::new(gettext("Initializing subsystems…")).unwrap();
        naevc::loadscreen_update(0., s.as_ptr());
    }

    // OpenAL
    unsafe {
        if naevc::conf.nosound != 0 {
            info!(gettext("Sound is disabled!"));
            naevc::sound_disabled = 1;
            naevc::music_disabled = 1;
        }
        if naevc::sound_init() != 0 {
            warn!(gettext("Problem setting up sound!"));
        }
        let m = CString::new("load").unwrap();
        naevc::music_choose(m.as_ptr());
    }

    // Misc Init
    unsafe {
        naevc::fps_setPos(
            15.,
            (naevc::gl_screen.h - 15 - naevc::gl_defFontMono.h) as f64,
        );

        // Misc graphics init
        naevc::render_init();
        naevc::nebu_init();
        naevc::gui_init();
        naevc::toolkit_init();
        naevc::map_init();
        naevc::map_system_init();
        naevc::cond_init();
        naevc::cli_init();

        // Load game data
        naevc::load_all();

        // Detect size changes that occurred during load.
        naevc::naev_resize();

        // Unload load screen.
        naevc::loadscreen_unload();
    }

    unsafe {
        naev_main();
    };
    Ok(())
    /*
    #if SDL_VERSION_ATLEAST( 3, 0, 0 )
          SDL_ShowSimpleMessageBox( SDL_MESSAGEBOX_ERROR,
                                    _( "Naev Critical Error" ), buf,
                                    gl_screen.window );
    #endif
            */
}
