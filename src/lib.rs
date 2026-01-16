use std::io::Cursor;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use anyhow::{Context, Result};

use rodio::Decoder;

use serde::{Deserialize, Serialize};

use paste::paste;

use binaryninja::binary_view::{
    BinaryView, BinaryViewEventHandler, BinaryViewEventType, StringType,
};
use binaryninja::component::Component;
use binaryninja::data_notification::{
    CustomDataNotification, DataNotificationTriggers,
};
use binaryninja::database::undo::UndoEntry;
use binaryninja::external_library::{ExternalLibrary, ExternalLocation};
use binaryninja::function::Function;
use binaryninja::section::Section;
use binaryninja::segment::Segment;
use binaryninja::symbol::Symbol;
use binaryninja::tags::{TagReference, TagType};
use binaryninja::types::{QualifiedName, Type, TypeArchive};
use binaryninja::variable::DataVariable;

const LOG_NAME: &str = "BinJuice";
macro_rules! log_dbg {
    ($msg:expr) => {
        #[cfg(debug_assertions)]
        binaryninja::logger::bn_log(
            LOG_NAME,
            binaryninja::logger::BnLogLevel::DebugLog,
            $msg,
        )
    };
}

macro_rules! info {
    ($msg:expr) => {
        binaryninja::logger::bn_log(
            LOG_NAME,
            binaryninja::logger::BnLogLevel::InfoLog,
            $msg,
        )
    };
}
macro_rules! warn {
    ($msg:expr) => {
        binaryninja::logger::bn_log(
            LOG_NAME,
            binaryninja::logger::BnLogLevel::WarningLog,
            $msg,
        )
    };
}
macro_rules! err {
    ($msg:expr) => {
        binaryninja::logger::bn_log(
            LOG_NAME,
            binaryninja::logger::BnLogLevel::ErrorLog,
            $msg,
        )
    };
}

static SOUND_HANDLER: OnceLock<SoundHandler> = OnceLock::new();
// TODO currently the binaryview used are stored like this, so we don't register
// the same bv multiple times, fix this
static BINVIEW_HANDLERS: Mutex<Vec<usize>> = Mutex::new(vec![]);

pub struct SoundHandler {
    stream_handle: rodio::OutputStream,
    audio: AudioFiles,
    _handles: Mutex<Vec<usize>>,
}

impl std::fmt::Debug for SoundHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SoundHandler").finish()
    }
}

impl SoundHandler {
    // this is called on UI creation event, AKA when binja opens
    pub fn new() -> Result<Self> {
        let config_file = binaryninja::user_directory().join("binjuice.yml");
        let file = std::fs::File::open(config_file)
            .context("Unable to open the config file")?;
        let config: Config = serde_yaml::from_reader(file)?;
        let audio = AudioFiles::load_files(&config.files)?;

        log_dbg!("Getting default output stream");
        let stream_handle = rodio::OutputStreamBuilder::open_default_stream()?;

        let slf = Self {
            stream_handle,
            audio,
            _handles: Mutex::new(vec![]),
        };
        slf.play_start_binary_ninja();
        Ok(slf)
    }

    fn play_audio(&self, audio: &Option<Arc<[u8]>>, name: &'static str) {
        #[cfg(debug_assertions)]
        log_dbg!(&format!("Audio callback for: {name}"));
        if let Some(audio) = audio.as_ref() {
            info!(&format!("Play audio file: {name}"));
            let decoder =
                match Decoder::try_from(Cursor::new(Arc::clone(&audio))) {
                    Ok(decoder) => decoder,
                    Err(e) => {
                        err!(&format!(
                            "Unable to decode audio for {name}: {e}"
                        ));
                        return;
                    }
                };
            self.stream_handle.mixer().add(decoder);
        }
    }
}

struct AnalysisCompletionEvent;
impl BinaryViewEventHandler for AnalysisCompletionEvent {
    // this is called when the auto analysis ends
    fn on_event(&self, view: &BinaryView) {
        log_dbg!("InitTrigger called");
        // makes sure this handles is not already attached
        if let Some(_old) = BINVIEW_HANDLERS
            .lock()
            .unwrap()
            .iter()
            .find(|x| **x == view.handle as usize)
        {
            // TODO is the address unique? what if the BinaryView is open and
            // closed? Could the address be reutilized?
            warn!("InitTrigger called multiple times on the same BinView");
            return;
        }

        BINVIEW_HANDLERS.lock().unwrap().push(view.handle as usize);
        // register the sound handler
        let sound_handler = SOUND_HANDLER
            .get()
            .expect("Plugin not initialized correctly");

        sound_handler.play_start_binary_view();

        // TODO don't leak this: https://github.com/Vector35/binaryninja-api/issues/7890
        let _handle = Box::leak(Box::new(
            sound_handler.register(view, sound_handler.triggers()),
        ));
        sound_handler
            ._handles
            .lock()
            .unwrap()
            .push(_handle as *const _ as usize);

        log_dbg!("InitTrigger registered");
    }
}

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub files: AudioConfig,
}

// just copied that from the binary ninja source code
macro_rules! trait_handler {
(
    [manual]
    $($manual_fun_name:ident),* $(,)?
    [ffi]
    $(
        $ffi_param_name:ident => $fun_name:ident(
            $(
                $arg_name:ident:
                $raw_arg_type:ty:
                $arg_type:ty =
                $value_calculated:expr
            ),* $(,)?
        ) $(-> $ret_type:ty)?
    ),* $(,)?
) => {
    #[derive(Deserialize, Serialize)]
    pub struct AudioConfig {
        $($manual_fun_name: Option<PathBuf>,)*
        $($fun_name: Option<PathBuf>,)*
    }

    // TODO if the RAM shortage don't get fixed
    // please update this to not store the raw
    // file data into RAW
    struct AudioFiles {
        // TODO implement the end functions...
        #[allow(unused)]
        $($manual_fun_name: Option<Arc<[u8]>>,)*
        $($fun_name: Option<Arc<[u8]>>,)*
    }

    impl AudioFiles {
        fn load_files(config: &AudioConfig) -> Result<Self> {
            fn read_all(path: &Option<PathBuf>) -> Result<Option<Arc<[u8]>>> {
                Ok(path.as_ref()
                    .map(std::fs::read)
                    .transpose()?
                    .map(Arc::from))
            }
            Ok(Self {
                $($manual_fun_name: read_all(&config.$manual_fun_name)?,)*
                $($fun_name: read_all(&config.$fun_name)?,)*
            })
        }
    }

    impl SoundHandler {
        fn triggers(&self) -> DataNotificationTriggers {
            let mut triggers = DataNotificationTriggers::default();
            $(
            if self.audio.$fun_name.is_some() {
                triggers = triggers.$fun_name();
            }
            )*

            triggers
        }

        // TODO implement the end functions...
        paste! {
        $(
        #[allow(unused)]
        fn [<play_ $manual_fun_name>](&self) {
            self.play_audio(&self.audio.$manual_fun_name, stringify!($manual_fun_name))
        }
        )*
        $(
        fn [<play_ $fun_name>](&self) {
            self.play_audio(&self.audio.$fun_name, stringify!($fun_name))
        }
        )*
        }
    }
    impl CustomDataNotification for &SoundHandler {
        $(
        fn $fun_name(&mut self, $(_: $arg_type),*) $(-> $ret_type)* {
            paste! {
                self.[<play_ $fun_name>]();
            }
            $( <$ret_type as Default>::default() )*
        }
        )*
    }
};
}
trait_handler! {
    [manual]
    start_binary_ninja,
    end_binary_ninja,
    start_binary_view,
    end_binary_view,
    [ffi]
    notificationBarrier => notification_barrier(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
    ) -> u64,
    dataWritten => data_written(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        offset: u64: u64 = offset,
        len: usize: usize = len,
    ),
    dataInserted => data_inserted(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        offset: u64: u64 = offset,
        len: usize: usize = len,
    ),
    dataRemoved => data_removed(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        offset: u64: u64 = offset,
        len: u64: u64 = len,
    ),
    functionAdded => function_added(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        func: *mut BNFunction: &Function = &Function::from_raw(func),
    ),
    functionRemoved => function_removed(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        func: *mut BNFunction: &Function = &Function::from_raw(func),
    ),
    functionUpdated => function_updated(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        func: *mut BNFunction: &Function = &Function::from_raw(func),
    ),
    functionUpdateRequested => function_update_requested(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        func: *mut BNFunction: &Function = &Function::from_raw(func),
    ),
    dataVariableAdded => data_variable_added(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        var: *mut BNDataVariable: &DataVariable = &DataVariable::from_raw(&*var),
    ),
    dataVariableRemoved => data_variable_removed(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        var: *mut BNDataVariable: &DataVariable = &DataVariable::from_raw(&*var),
    ),
    dataVariableUpdated => data_variable_updated(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        var: *mut BNDataVariable: &DataVariable = &DataVariable::from_raw(&*var),
    ),
    dataMetadataUpdated => data_metadata_updated(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        offset: u64: u64 = offset,
    ),
    tagTypeUpdated => tag_type_updated(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        tag_type: *mut BNTagType: &TagType = &TagType{ handle: tag_type },
    ),
    tagAdded => tag_added(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        tag_ref: *mut BNTagReference: &TagReference = &TagReference::from(&*tag_ref),
    ),
    tagRemoved => tag_removed(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        tag_ref: *mut BNTagReference: &TagReference = &TagReference::from(&*tag_ref),
    ),
    tagUpdated => tag_updated(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        tag_ref: *mut BNTagReference: &TagReference = &TagReference::from(&*tag_ref),
    ),
    symbolAdded => symbol_added(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        sym: *mut BNSymbol: &Symbol = &Symbol::from_raw(sym),
    ),
    symbolRemoved => symbol_removed(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        sym: *mut BNSymbol: &Symbol = &Symbol::from_raw(sym),
    ),
    symbolUpdated => symbol_updated(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        sym: *mut BNSymbol: &Symbol = &Symbol::from_raw(sym),
    ),
    stringFound => string_found(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        type_: BNStringType: StringType = type_,
        offset: u64: u64 = offset,
        len: usize: usize = len,
    ),
    stringRemoved => string_removed(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        type_: BNStringType: StringType = type_,
        offset: u64: u64 = offset,
        len: usize: usize = len,
    ),
    typeDefined => type_defined(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        name: *mut BNQualifiedName: &QualifiedName = &QualifiedName::from_raw(&*name),
        type_: *mut BNType: &Type = &Type::from_raw(type_),
    ),
    typeUndefined => type_undefined(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        name: *mut BNQualifiedName: &QualifiedName = &QualifiedName::from_raw(&*name),
        type_: *mut BNType: &Type = &Type::from_raw(type_),
    ),
    typeReferenceChanged => type_reference_changed(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        name: *mut BNQualifiedName: &QualifiedName = &QualifiedName::from_raw(&*name),
        type_: *mut BNType: &Type = &Type::from_raw(type_),
    ),
    typeFieldReferenceChanged => type_field_reference_changed(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        name: *mut BNQualifiedName: &QualifiedName = &QualifiedName::from_raw(&*name),
        offset: u64: u64 = offset,
    ),
    segmentAdded => segment_added(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        segment: *mut BNSegment: &Segment = &Segment::from_raw(segment),
    ),
    segmentRemoved => segment_removed(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        segment: *mut BNSegment: &Segment = &Segment::from_raw(segment),
    ),
    segmentUpdated => segment_updated(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        segment: *mut BNSegment: &Segment = &Segment::from_raw(segment),
    ),
    sectionAdded => section_added(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        section: *mut BNSection: &Section = &Section::from_raw(section),
    ),
    sectionRemoved => section_removed(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        section: *mut BNSection: &Section = &Section::from_raw(section),
    ),
    sectionUpdated => section_updated(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        section: *mut BNSection: &Section = &Section::from_raw(section),
    ),
    componentNameUpdated => component_name_updated(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        previous_name: *mut c_char: &str = CStr::from_ptr(previous_name).to_str().unwrap(),
        component: *mut BNComponent: &Component = &Component::from_raw(NonNull::new(component).unwrap()),
    ),
    componentAdded => component_added(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        component: *mut BNComponent: &Component = &Component::from_raw(NonNull::new(component).unwrap()),
    ),
    componentMoved => component_moved(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        former_parent: *mut BNComponent: &Component = &Component::from_raw(NonNull::new(former_parent).unwrap()),
        new_parent: *mut BNComponent: &Component = &Component::from_raw(NonNull::new(new_parent).unwrap()),
        component: *mut BNComponent: &Component = &Component::from_raw(NonNull::new(component).unwrap()),
    ),
    componentRemoved => component_removed(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        former_parent: *mut BNComponent: &Component = &Component::from_raw(NonNull::new(former_parent).unwrap()),
        component: *mut BNComponent: &Component = &Component::from_raw(NonNull::new(component).unwrap()),
    ),
    componentFunctionAdded => component_function_added(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        component: *mut BNComponent: &Component = &Component::from_raw(NonNull::new(component).unwrap()),
        function: *mut BNFunction: &Function = &Function::from_raw(function),
    ),
    componentFunctionRemoved => component_function_removed(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        component: *mut BNComponent: &Component = &Component::from_raw(NonNull::new(component).unwrap()),
        function: *mut BNFunction: &Function = &Function::from_raw(function),
    ),
    componentDataVariableAdded => component_data_variable_added(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        component: *mut BNComponent: &Component = &Component::from_raw(NonNull::new(component).unwrap()),
        var: *mut BNDataVariable: &DataVariable = &DataVariable::from_raw(&*var),
        ),
    componentDataVariableRemoved => component_data_variable_removed(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        component: *mut BNComponent: &Component = &Component::from_raw(NonNull::new(component).unwrap()),
        var: *mut BNDataVariable: &DataVariable = &DataVariable::from_raw(&*var),
    ),
    externalLibraryAdded => external_library_added(
        data: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(data),
        library: *mut BNExternalLibrary: &ExternalLibrary = &ExternalLibrary::from_raw(NonNull::new(library).unwrap()),
    ),
    externalLibraryUpdated => external_library_updated(
        data: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(data),
        library: *mut BNExternalLibrary: &ExternalLibrary = &ExternalLibrary::from_raw(NonNull::new(library).unwrap()),
    ),
    externalLibraryRemoved => external_library_removed(
        data: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(data),
        library: *mut BNExternalLibrary: &ExternalLibrary = &ExternalLibrary::from_raw(NonNull::new(library).unwrap()),
    ),
    externalLocationAdded => external_location_added(
        data: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(data),
        location: *mut BNExternalLocation: &ExternalLocation = &ExternalLocation::from_raw(NonNull::new(location).unwrap()),
    ),
    externalLocationUpdated => external_location_updated(
        data: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(data),
        location: *mut BNExternalLocation: &ExternalLocation = &ExternalLocation::from_raw(NonNull::new(location).unwrap()),
    ),
    externalLocationRemoved => external_location_removed(
        data: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(data),
        location: *mut BNExternalLocation: &ExternalLocation = &ExternalLocation::from_raw(NonNull::new(location).unwrap()),
    ),
    typeArchiveAttached => type_archive_attached(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        id: *const c_char: &str = CStr::from_ptr(id).to_str().unwrap(),
        path: *const c_char: &[u8] = CStr::from_ptr(path).to_bytes(),
    ),
    typeArchiveDetached => type_archive_detached(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        id: *const c_char: &str = CStr::from_ptr(id).to_str().unwrap(),
        path: *const c_char: &[u8] = CStr::from_ptr(path).to_bytes(),
    ),
    typeArchiveConnected => type_archive_connected(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        archive: *mut BNTypeArchive: &TypeArchive = &TypeArchive::from_raw(NonNull::new(archive).unwrap()),
    ),
    typeArchiveDisconnected => type_archive_disconnected(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        archive: *mut BNTypeArchive: &TypeArchive = &TypeArchive::from_raw(NonNull::new(archive).unwrap()),
    ),
    undoEntryAdded => undo_entry_added(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        entry: *mut BNUndoEntry: &UndoEntry = &UndoEntry::from_raw(NonNull::new(entry).unwrap()),
    ),
    undoEntryTaken => undo_entry_taken(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        entry: *mut BNUndoEntry: &UndoEntry = &UndoEntry::from_raw(NonNull::new(entry).unwrap()),
    ),
    redoEntryTaken => redo_entry_taken(
        view: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(view),
        entry: *mut BNUndoEntry: &UndoEntry = &UndoEntry::from_raw(NonNull::new(entry).unwrap()),
    ),
    rebased => rebased(
        oldview: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(oldview),
        newview: *mut BNBinaryView: &BinaryView = &BinaryView::from_raw(newview),
    ),
}

#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "C" fn UIPluginInit() -> bool {
    binaryninja::tracing_init!(LOG_NAME);

    // create the logger, so it shows up at the log "filter" dropdown
    let _ = binaryninja::logger::Logger::new(LOG_NAME);

    let handler = match SoundHandler::new() {
        Ok(handler) => handler,
        Err(err) => {
            err!(&format!("Unable init BinJuice sound handler: {err}"));
            return false;
        }
    };

    SOUND_HANDLER
        .set(handler)
        .expect("BinJuice was initialized multiple times");
    binaryninja::binary_view::register_binary_view_event(
        BinaryViewEventType::BinaryViewInitialAnalysisCompletionEvent,
        AnalysisCompletionEvent,
    );

    true
}
