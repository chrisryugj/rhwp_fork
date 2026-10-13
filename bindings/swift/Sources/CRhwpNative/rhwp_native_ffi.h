#ifndef RHWP_NATIVE_FFI_H
#define RHWP_NATIVE_FFI_H

#ifdef __cplusplus
extern "C" {
#endif

char *rhwp_export_text(const char *input_path, const char *output_dir, int page);
char *rhwp_export_markdown(const char *input_path, const char *output_dir, int page);
/* Print-profile direct PDF; page=-1 means all, otherwise zero-based.
 * Input <=128 MiB, selected pages <=256. Destination must not exist.
 * All pages are rendered before creating the file. Requires native-skia;
 * valid arguments in disabled builds return {"ok":false,"code":"featureDisabled",...}.
 * Free the returned UTF-8 JSON with rhwp_string_free. */
char *rhwp_export_pdf_direct(const char *input_path, const char *output_file, int page);
char *rhwp_read_text(const char *input_path, int page);
void rhwp_string_free(char *value);

#ifdef __cplusplus
}
#endif

#endif
