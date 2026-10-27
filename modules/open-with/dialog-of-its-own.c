/*
 * A GTK 3 program opening its own dialog for choosing a file, for the
 * gtk-3-dialogs-ask-the-portal check (dialogs-asked.py).
 *
 * `dialog-of-its-own open|save <folder> [<name>]` opens a GtkFileChooserDialog
 * to open a file or to save one as <name> in <folder>, and says on its own
 * line "shown" if the dialog ever shows, then "answered <response> <file>"
 * when it answers, the file being "-" when it says none was chosen. Opening
 * takes GTK_RESPONSE_ACCEPT for its accept button, and saving
 * GTK_RESPONSE_OK, as programs use either.
 */

#include <gtk/gtk.h>
#include <string.h>

static void
shown (G_GNUC_UNUSED GtkWidget *dialog, G_GNUC_UNUSED gpointer unused)
{
  g_print ("shown\n");
}

static void
answered (GtkDialog *dialog, gint response, G_GNUC_UNUSED gpointer unused)
{
  gchar *file = gtk_file_chooser_get_filename (GTK_FILE_CHOOSER (dialog));

  g_print ("answered %d %s\n", response, file != NULL ? file : "-");
  g_free (file);
  gtk_main_quit ();
}

static gboolean
unanswered (G_GNUC_UNUSED gpointer unused)
{
  g_print ("unanswered\n");
  gtk_main_quit ();
  return G_SOURCE_REMOVE;
}

int
main (int argc, char **argv)
{
  gboolean saving;
  GtkWidget *dialog;

  gtk_init (&argc, &argv);
  if (argc < 3)
    return 2;
  saving = strcmp (argv[1], "save") == 0;
  if (saving)
    dialog = gtk_file_chooser_dialog_new ("Save a draft", NULL, GTK_FILE_CHOOSER_ACTION_SAVE,
                                          "_Cancel", GTK_RESPONSE_CANCEL, "_Save",
                                          GTK_RESPONSE_OK, NULL);
  else
    dialog = gtk_file_chooser_dialog_new ("Open a picture", NULL, GTK_FILE_CHOOSER_ACTION_OPEN,
                                          "_Cancel", GTK_RESPONSE_CANCEL, "_Open",
                                          GTK_RESPONSE_ACCEPT, NULL);
  gtk_file_chooser_set_current_folder (GTK_FILE_CHOOSER (dialog), argv[2]);
  if (saving && argc > 3)
    gtk_file_chooser_set_current_name (GTK_FILE_CHOOSER (dialog), argv[3]);
  g_signal_connect (dialog, "map", G_CALLBACK (shown), NULL);
  g_signal_connect (dialog, "response", G_CALLBACK (answered), NULL);
  gtk_widget_show (dialog);
  g_timeout_add_seconds (30, unanswered, NULL);
  gtk_main ();
  return 0;
}
