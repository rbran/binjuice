BinJuice is a Binary Ninja plugin that play sounds using the `DataNotification` interface.

[![BinJuice in Action](https://img.youtube.com/vi/t9W2C3xB5_A/hqdefault.jpg)](https://www.youtube.com/embed/t9W2C3xB5_A)

This is a joke and should not be used in production.

But if you want to use it anyways, you can configure it by creating the file at `${BINJA_USER_DIR}/binjuice.yml`.

It uses [rodio](https://crates.io/crates/rodio), so by default it supports FLAC, MP3, Vorbis and WAV. 

The available options are:

```yaml
files:
  start_binary_ninja: /path/to/your/audio/01.flac
  end_binary_ninja: /path/to/your/audio/02.flac
  start_binary_view: /path/to/your/audio/03.flac
  end_binary_view: /path/to/your/audio/04.flac
  notification_barrier: /path/to/your/audio/05.flac
  data_written: /path/to/your/audio/06.flac
  data_inserted: /path/to/your/audio/07.flac
  data_removed: /path/to/your/audio/08.flac
  function_added: /path/to/your/audio/09.flac
  function_removed: /path/to/your/audio/10.flac
  function_updated: /path/to/your/audio/11.flac
  function_update_requested: /path/to/your/audio/12.flac
  data_variable_added: /path/to/your/audio/13.flac
  data_variable_removed: /path/to/your/audio/14.flac
  data_variable_updated: /path/to/your/audio/15.flac
  data_metadata_updated: /path/to/your/audio/16.flac
  tag_type_updated: /path/to/your/audio/17.flac
  tag_added: /path/to/your/audio/18.flac
  tag_removed: /path/to/your/audio/19.flac
  tag_updated: /path/to/your/audio/20.flac
  symbol_added: /path/to/your/audio/21.flac
  symbol_removed: /path/to/your/audio/22.flac
  symbol_updated: /path/to/your/audio/23.flac
  string_found: /path/to/your/audio/24.flac
  string_removed: /path/to/your/audio/25.flac
  type_defined: /path/to/your/audio/26.flac
  type_undefined: /path/to/your/audio/27.flac
  type_reference_changed: /path/to/your/audio/28.flac
  type_field_reference_changed: /path/to/your/audio/29.flac
  segment_added: /path/to/your/audio/30.flac
  segment_removed: /path/to/your/audio/31.flac
  segment_updated: /path/to/your/audio/32.flac
  section_added: /path/to/your/audio/33.flac
  section_removed: /path/to/your/audio/34.flac
  section_updated: /path/to/your/audio/35.flac
  component_name_updated: /path/to/your/audio/36.flac
  component_added: /path/to/your/audio/37.flac
  component_moved: /path/to/your/audio/38.flac
  component_removed: /path/to/your/audio/39.flac
  component_function_added: /path/to/your/audio/40.flac
  component_function_removed: /path/to/your/audio/41.flac
  component_data_variable_added: /path/to/your/audio/42.flac
  component_data_variable_removed: /path/to/your/audio/43.flac
  external_library_added: /path/to/your/audio/44.flac
  external_library_updated: /path/to/your/audio/45.flac
  external_library_removed: /path/to/your/audio/46.flac
  external_location_added: /path/to/your/audio/47.flac
  external_location_updated: /path/to/your/audio/48.flac
  external_location_removed: /path/to/your/audio/49.flac
  type_archive_attached: /path/to/your/audio/50.flac
  type_archive_detached: /path/to/your/audio/11.flac
  type_archive_connected: /path/to/your/audio/12.flac
  type_archive_disconnected: /path/to/your/audio/13.flac
  undo_entry_added: /path/to/your/audio/14.flac
  undo_entry_taken: /path/to/your/audio/05.flac
  redo_entry_taken: /path/to/your/audio/16.flac
  rebased: /path/to/your/audio/17.flac
```
