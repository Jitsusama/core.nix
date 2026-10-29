/*
 * A GTK 3 program's own dialog for choosing a file asks the portal instead.
 *
 * GTK 3 draws a GtkFileChooserDialog itself, and only its native chooser asks
 * the portal, whose file chooser is open-with, which asks the launcher. Loaded
 * into every GTK 3 program through GTK3_MODULES, this keeps such a dialog from
 * showing. As it would show, a native chooser is asked in its place, with the
 * dialog's title, action, folder, name and filters; the files chosen are what
 * the dialog then says was chosen, and the dialog answers as its own accept
 * button would, so the program carries on as if its dialog had been used.
 * Cancelled, the dialog answers as if it were closed.
 *
 * It's C, not Rust, because it's loaded into the program as a GTK module, and
 * reaches into GTK's own classes, which GTK's own language says most plainly.
 */

#include <gtk/gtk.h>
#include <string.h>

/*
 * The start of GtkFileChooserIface, which GTK keeps private
 * (gtk/gtkfilechooserprivate.h in GTK 3.24). GTK 3 is finished, so its layout
 * holds; the check of this module proves the dialog answers what was chosen.
 */
typedef struct
{
  GTypeInterface base_iface;
  gboolean (*set_current_folder) (GtkFileChooser *, GFile *, GError **);
  GFile *(*get_current_folder) (GtkFileChooser *);
  void (*set_current_name) (GtkFileChooser *, const gchar *);
  gchar *(*get_current_name) (GtkFileChooser *);
  gboolean (*select_file) (GtkFileChooser *, GFile *, GError **);
  void (*unselect_file) (GtkFileChooser *, GFile *);
  void (*select_all) (GtkFileChooser *);
  void (*unselect_all) (GtkFileChooser *);
  GSList *(*get_files) (GtkFileChooser *);
} Chooser;

/* What the portal chose, kept on the dialog until it's asked again. */
#define CHOSEN "dialogs-ask-the-portal-chosen"
/* The native chooser asked for a dialog, while it's asking. */
#define ASKING "dialogs-ask-the-portal-asking"
/* The folder the program last asked the dialog to show. GTK takes a
 * folder only once it has read that it is one, so a dialog shown straight
 * after being given one would otherwise say it has none. */
#define WITHIN "dialogs-ask-the-portal-within"

/* GTK's own, which a dialog nothing was chosen in still answers with. */
static void (*shown) (GtkWidget *);
static gboolean (*moved) (GtkFileChooser *, GFile *, GError **);
static gboolean (*picked) (GtkFileChooser *, GFile *, GError **);
static GFile *(*folder) (GtkFileChooser *);
static gchar *(*named) (GtkFileChooser *);
static GSList *(*listed) (GtkFileChooser *);

/* Whether a dialog being shown is one the native chooser opened itself,
 * as it does when the portal can't be reached. */
static gboolean in_native;

static void
let_go (gpointer files)
{
  g_slist_free_full (files, g_object_unref);
}

static gpointer
kept (gconstpointer file, G_GNUC_UNUSED gpointer unused)
{
  return g_object_ref ((gpointer) file);
}

static GSList *
chosen (GtkFileChooser *chooser)
{
  return g_object_get_data (G_OBJECT (chooser), CHOSEN);
}

static GSList *
chosen_files (GtkFileChooser *chooser)
{
  GSList *files = chosen (chooser);

  if (files == NULL)
    return listed (chooser);
  return g_slist_copy_deep (files, kept, NULL);
}

static gboolean
move (GtkFileChooser *chooser, GFile *within, GError **error)
{
  g_object_set_data_full (G_OBJECT (chooser), WITHIN, g_object_ref (within), g_object_unref);
  return moved (chooser, within, error);
}

static gboolean
pick (GtkFileChooser *chooser, GFile *file, GError **error)
{
  GFile *within = g_file_get_parent (file);

  if (within != NULL)
    g_object_set_data_full (G_OBJECT (chooser), WITHIN, within, g_object_unref);
  return picked (chooser, file, error);
}

static GFile *
chosen_folder (GtkFileChooser *chooser)
{
  GSList *files = chosen (chooser);

  if (files == NULL)
    return folder (chooser);
  if (gtk_file_chooser_get_action (chooser) == GTK_FILE_CHOOSER_ACTION_SELECT_FOLDER)
    return g_object_ref (files->data);
  return g_file_get_parent (files->data);
}

static gchar *
chosen_name (GtkFileChooser *chooser)
{
  GSList *files = chosen (chooser);

  if (files == NULL)
    return named (chooser);
  return g_file_get_basename (files->data);
}

/* What the dialog's accept button answers, as GTK reads one, else ACCEPT. */
static gint
accepting (GtkDialog *dialog)
{
  static const gint accepts[] = {
    GTK_RESPONSE_ACCEPT,
    GTK_RESPONSE_OK,
    GTK_RESPONSE_YES,
    GTK_RESPONSE_APPLY,
  };
  gsize each;

  for (each = 0; each < G_N_ELEMENTS (accepts); each++)
    if (gtk_dialog_get_widget_for_response (dialog, accepts[each]) != NULL)
      return accepts[each];
  return GTK_RESPONSE_ACCEPT;
}

/* The dialog answers for what was chosen as its accept button would. Its
 * own first handler of an answer, which GTK connects as it's made, would
 * refuse it for the dialog's empty selection, so it's held back while the
 * program's own handlers hear it. Its button can't be pressed instead,
 * since a button never shown never activates. */
static void
accept (GtkDialog *dialog)
{
  guint answers = g_signal_lookup ("response", GTK_TYPE_DIALOG);
  gulong own = g_signal_handler_find (dialog, G_SIGNAL_MATCH_ID, answers, 0, NULL, NULL, NULL);

  if (own != 0)
    g_signal_handler_block (dialog, own);
  gtk_dialog_response (dialog, accepting (dialog));
  if (own != 0 && g_signal_handler_is_connected (dialog, own))
    g_signal_handler_unblock (dialog, own);
}

static void
heard (GtkNativeDialog *native, gint response, gpointer data)
{
  GtkDialog *dialog = data;
  GtkFileFilter *filter;

  g_object_ref (dialog);
  g_object_steal_data (G_OBJECT (dialog), ASKING);
  if (response == GTK_RESPONSE_ACCEPT)
    {
      g_object_set_data_full (G_OBJECT (dialog), CHOSEN,
                              gtk_file_chooser_get_files (GTK_FILE_CHOOSER (native)), let_go);
      filter = gtk_file_chooser_get_filter (GTK_FILE_CHOOSER (native));
      if (filter != NULL)
        gtk_file_chooser_set_filter (GTK_FILE_CHOOSER (dialog), filter);
    }
  gtk_native_dialog_destroy (native);
  g_object_unref (native);
  if (response == GTK_RESPONSE_ACCEPT && chosen (GTK_FILE_CHOOSER (dialog)) != NULL)
    accept (dialog);
  else
    gtk_dialog_response (dialog, GTK_RESPONSE_DELETE_EVENT);
  g_object_unref (dialog);
}

static void
forget (gpointer native)
{
  g_signal_handlers_disconnect_matched (native, G_SIGNAL_MATCH_FUNC, 0, 0, NULL, heard, NULL);
  gtk_native_dialog_destroy (native);
  g_object_unref (native);
}

/* The native chooser, asked what the dialog would have asked. */
static GtkFileChooserNative *
asked (GtkWidget *dialog)
{
  GtkFileChooser *chooser = GTK_FILE_CHOOSER (dialog);
  GtkFileChooserAction action = gtk_file_chooser_get_action (chooser);
  GtkFileChooserNative *native;
  GSList *filters, *each;
  GFile *at;
  gchar *name;

  native = gtk_file_chooser_native_new (gtk_window_get_title (GTK_WINDOW (dialog)),
                                        gtk_window_get_transient_for (GTK_WINDOW (dialog)),
                                        action, NULL, NULL);
  gtk_native_dialog_set_modal (GTK_NATIVE_DIALOG (native),
                               gtk_window_get_modal (GTK_WINDOW (dialog)));
  gtk_file_chooser_set_select_multiple (GTK_FILE_CHOOSER (native),
                                        gtk_file_chooser_get_select_multiple (chooser));
  gtk_file_chooser_set_do_overwrite_confirmation (
    GTK_FILE_CHOOSER (native), gtk_file_chooser_get_do_overwrite_confirmation (chooser));
  at = g_object_get_data (G_OBJECT (dialog), WITHIN);
  at = at != NULL ? g_object_ref (at) : gtk_file_chooser_get_current_folder_file (chooser);
  if (at != NULL)
    {
      gtk_file_chooser_set_current_folder_file (GTK_FILE_CHOOSER (native), at, NULL);
      g_object_unref (at);
    }
  if (action == GTK_FILE_CHOOSER_ACTION_SAVE)
    {
      name = gtk_file_chooser_get_current_name (chooser);
      if (name != NULL && *name != '\0')
        gtk_file_chooser_set_current_name (GTK_FILE_CHOOSER (native), name);
      g_free (name);
    }
  filters = gtk_file_chooser_list_filters (chooser);
  for (each = filters; each != NULL; each = each->next)
    gtk_file_chooser_add_filter (GTK_FILE_CHOOSER (native), each->data);
  g_slist_free (filters);
  if (gtk_file_chooser_get_filter (chooser) != NULL)
    gtk_file_chooser_set_filter (GTK_FILE_CHOOSER (native),
                                 gtk_file_chooser_get_filter (chooser));
  return native;
}

static void
show (GtkWidget *dialog)
{
  GtkFileChooserNative *native;

  if (in_native)
    {
      shown (dialog);
      return;
    }
  if (g_object_get_data (G_OBJECT (dialog), ASKING) != NULL)
    return;
  g_object_set_data (G_OBJECT (dialog), CHOSEN, NULL);
  native = asked (dialog);
  g_object_set_data_full (G_OBJECT (dialog), ASKING, native, forget);
  g_signal_connect (native, "response", G_CALLBACK (heard), dialog);
  in_native = TRUE;
  gtk_native_dialog_show (GTK_NATIVE_DIALOG (native));
  in_native = FALSE;
}

G_MODULE_EXPORT void gtk_module_init (gint *argc, gchar ***argv);

G_MODULE_EXPORT void
gtk_module_init (G_GNUC_UNUSED gint *argc, G_GNUC_UNUSED gchar ***argv)
{
  GtkWidgetClass *dialogs;
  Chooser *chooser;

  /* GTK's own portal draws these dialogs for the portal; asking the portal
   * from it would ask itself. The native chooser asks the portal only when
   * GTK_USE_PORTAL says to, and would otherwise draw one of these. */
  if (strstr (g_get_prgname () ? g_get_prgname () : "", "xdg-desktop-portal") != NULL
      || g_strcmp0 (g_getenv ("GTK_USE_PORTAL"), "1") != 0)
    return;

  dialogs = g_type_class_ref (GTK_TYPE_FILE_CHOOSER_DIALOG);
  shown = dialogs->show;
  dialogs->show = show;

  chooser = g_type_interface_peek (dialogs, GTK_TYPE_FILE_CHOOSER);
  moved = chooser->set_current_folder;
  chooser->set_current_folder = move;
  picked = chooser->select_file;
  chooser->select_file = pick;
  folder = chooser->get_current_folder;
  chooser->get_current_folder = chosen_folder;
  named = chooser->get_current_name;
  chooser->get_current_name = chosen_name;
  listed = chooser->get_files;
  chooser->get_files = chosen_files;
}
