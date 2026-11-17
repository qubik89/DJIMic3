//! UI language for the parts of the app drawn natively (tray menu and the
//! macOS app menu). The webview handles its own strings; it tells us the
//! chosen language through `set_locale`. Presentation only: device commands
//! and protocol values never change.

use std::sync::atomic::{AtomicU8, Ordering};
use tauri::{AppHandle, Manager};

/// Index into [`LOCALES`]; `0` is English.
#[derive(Default)]
pub struct LocaleState(pub AtomicU8);

const LOCALES: [&str; 6] = ["en", "es-ES", "fr", "it", "de", "nl"];

/// English source → [es-ES, fr, it, de, nl].
const TRAY: &[(&str, [&str; 5])] = &[
    ("Open DJI Mic Control", ["Abrir DJI Mic Control", "Ouvrir DJI Mic Control", "Apri DJI Mic Control", "DJI Mic Control öffnen", "DJI Mic Control openen"]),
    ("Start at login (hidden)", ["Abrir al iniciar sesión (oculta)", "Lancer à l'ouverture de session (masqué)", "Avvia al login (nascosta)", "Bei Anmeldung starten (ausgeblendet)", "Starten bij inloggen (verborgen)"]),
    ("Quit", ["Salir", "Quitter", "Esci", "Beenden", "Stoppen"]),
    ("Buy Me a Coffee…", ["Invítame a un café…", "Offrez-moi un café…", "Offrimi un caffè…", "Spendier mir einen Kaffee…", "Trakteer me op een koffie…"]),
    ("Mic LEDs: On  →  Off", ["LED: activados  →  desactivados", "LED : activées  →  désactivées", "LED: accesi  →  spenti", "Mikrofon-LEDs: Ein  →  Aus", "Microfoon-leds: aan  →  uit"]),
    ("Mic LEDs: Off  →  On", ["LED: desactivados  →  activados", "LED : désactivées  →  activées", "LED: spenti  →  accesi", "Mikrofon-LEDs: Aus  →  Ein", "Microfoon-leds: uit  →  aan"]),
    ("Toggle Mic LEDs", ["Activar/desactivar LED", "Activer/désactiver les LED", "Attiva/disattiva LED", "Mikrofon-LEDs umschalten", "Microfoon-leds schakelen"]),
    ("Noise Cancel: Strong  →  Basic", ["Cancelación de ruido: fuerte  →  básica", "Réduction de bruit : forte  →  basique", "Cancellazione rumore: forte  →  base", "Rauschunterdrückung: Stark  →  Basis", "Ruisonderdrukking: sterk  →  basis"]),
    ("Noise Cancel: Basic  →  Strong", ["Cancelación de ruido: básica  →  fuerte", "Réduction de bruit : basique  →  forte", "Cancellazione rumore: base  →  forte", "Rauschunterdrückung: Basis  →  Stark", "Ruisonderdrukking: basis  →  sterk"]),
    ("Toggle NC Mode (Basic / Strong)", ["Cambiar modo de cancelación (básica / fuerte)", "Changer le mode de réduction (basique / forte)", "Cambia modalità (base / forte)", "RU-Modus umschalten (Basis / Stark)", "Ruisonderdrukkingsmodus wisselen (basis / sterk)"]),
    ("Noise Cancelling: On  →  Off", ["Cancelación de ruido: activada  →  desactivada", "Réduction de bruit : activée  →  désactivée", "Cancellazione rumore: attiva  →  disattiva", "Rauschunterdrückung: Ein  →  Aus", "Ruisonderdrukking: aan  →  uit"]),
    ("Noise Cancelling: Off  →  On", ["Cancelación de ruido: desactivada  →  activada", "Réduction de bruit : désactivée  →  activée", "Cancellazione rumore: disattiva  →  attiva", "Rauschunterdrückung: Aus  →  Ein", "Ruisonderdrukking: uit  →  aan"]),
    ("Toggle Noise Cancelling", ["Activar/desactivar cancelación de ruido", "Activer/désactiver la réduction de bruit", "Attiva/disattiva cancellazione rumore", "Rauschunterdrückung umschalten", "Ruisonderdrukking schakelen"]),
];

fn index(app: &AppHandle) -> usize {
    app.state::<LocaleState>().0.load(Ordering::Relaxed) as usize
}

/// Translate a tray-menu label into the current UI language.
pub fn tr(app: &AppHandle, english: &'static str) -> &'static str {
    let i = index(app);
    if i == 0 {
        return english;
    }
    TRAY.iter()
        .find(|(en, _)| *en == english)
        .map(|(_, t)| t[i - 1])
        .unwrap_or(english)
}

#[tauri::command]
pub fn set_locale(app: AppHandle, locale: String) -> Result<(), String> {
    let i = LOCALES
        .iter()
        .position(|l| *l == locale)
        .ok_or_else(|| format!("Unsupported UI locale: {locale}"))?;
    let handle = app.clone();
    app.run_on_main_thread(move || {
        handle.state::<LocaleState>().0.store(i as u8, Ordering::Relaxed);
        super::set_tray_menu(&handle, &super::menu_state(&handle));
        #[cfg(target_os = "macos")]
        if let Ok(menu) = macos_menu(&handle, i) {
            let _ = handle.set_menu(menu);
        }
    })
    .map_err(|e| e.to_string())
}

/// The default macOS app menu with its labels translated, plus "Buy Me a
/// Coffee…" under About. Roles, shortcuts, Services and About metadata are
/// preserved since only the text changes.
#[cfg(target_os = "macos")]
fn macos_menu(app: &AppHandle, i: usize) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{Menu, MenuItem, MenuItemKind};

    /// English → [es-ES, fr, it, de, nl].
    const LABELS: &[(&str, [&str; 5])] = &[
        ("File", ["Archivo", "Fichier", "Archivio", "Ablage", "Archief"]),
        ("Edit", ["Edición", "Édition", "Modifica", "Bearbeiten", "Wijzig"]),
        ("View", ["Visualización", "Présentation", "Vista", "Darstellung", "Weergave"]),
        ("Window", ["Ventana", "Fenêtre", "Finestra", "Fenster", "Venster"]),
        ("Help", ["Ayuda", "Aide", "Aiuto", "Hilfe", "Help"]),
        ("Services", ["Servicios", "Services", "Servizi", "Dienste", "Voorzieningen"]),
        ("Undo", ["Deshacer", "Annuler", "Annulla", "Widerrufen", "Herstel"]),
        ("Redo", ["Rehacer", "Rétablir", "Ripeti", "Wiederholen", "Opnieuw"]),
        ("Cut", ["Cortar", "Couper", "Taglia", "Ausschneiden", "Knip"]),
        ("Copy", ["Copiar", "Copier", "Copia", "Kopieren", "Kopieer"]),
        ("Paste", ["Pegar", "Coller", "Incolla", "Einsetzen", "Plak"]),
        ("Select All", ["Seleccionar todo", "Tout sélectionner", "Seleziona tutto", "Alles auswählen", "Selecteer alles"]),
        ("Minimize", ["Minimizar", "Réduire", "Riduci a icona", "Im Dock ablegen", "Minimaliseer"]),
        ("Zoom", ["Zoom", "Zoom", "Zoom", "Zoomen", "Zoom"]),
        ("Maximize", ["Zoom", "Zoom", "Zoom", "Zoomen", "Zoom"]),
        ("Close", ["Cerrar ventana", "Fermer la fenêtre", "Chiudi finestra", "Fenster schließen", "Sluit venster"]),
        ("Close Window", ["Cerrar ventana", "Fermer la fenêtre", "Chiudi finestra", "Fenster schließen", "Sluit venster"]),
        ("Enter Full Screen", ["Pantalla completa", "Plein écran", "Schermo intero", "Vollbild", "Schermvullende weergave"]),
        ("Toggle Fullscreen", ["Pantalla completa", "Plein écran", "Schermo intero", "Vollbild", "Schermvullende weergave"]),
        ("Hide Others", ["Ocultar otros", "Masquer les autres", "Nascondi altre", "Andere ausblenden", "Verberg andere"]),
        ("Show All", ["Mostrar todo", "Tout afficher", "Mostra tutte", "Alle einblenden", "Toon alles"]),
    ];
    /// "About/Hide/Quit {app}" patterns → [es-ES, fr, it, de, nl].
    const NAMED: &[(&str, [&str; 5])] = &[
        ("About ", ["Acerca de {}", "À propos de {}", "Informazioni su {}", "Über {}", "Over {}"]),
        ("Hide ", ["Ocultar {}", "Masquer {}", "Nascondi {}", "{} ausblenden", "Verberg {}"]),
        ("Quit ", ["Salir de {}", "Quitter {}", "Esci da {}", "{} beenden", "Stop {}"]),
    ];

    let label = |text: &str| -> String {
        if let Some((_, t)) = LABELS.iter().find(|(en, _)| *en == text) {
            return t[i - 1].to_string();
        }
        for (prefix, t) in NAMED {
            if let Some(name) = text.strip_prefix(prefix) {
                return t[i - 1].replace("{}", name);
            }
        }
        text.to_string()
    };
    fn translate(items: Vec<MenuItemKind<tauri::Wry>>, label: &dyn Fn(&str) -> String) -> tauri::Result<()> {
        for item in items {
            match item {
                MenuItemKind::Submenu(menu) => {
                    menu.set_text(label(&menu.text()?))?;
                    translate(menu.items()?, label)?;
                }
                MenuItemKind::Predefined(item) => item.set_text(label(&item.text()?))?,
                MenuItemKind::MenuItem(item) => item.set_text(label(&item.text()?))?,
                _ => {}
            }
        }
        Ok(())
    }

    let menu = Menu::default(app)?;
    if i != 0 {
        translate(menu.items()?, &label)?;
    }
    // The first submenu is the one named after the app; About is its first item.
    // Clicks land in the tray's menu handler, which receives every menu event.
    if let Some(MenuItemKind::Submenu(app_menu)) = menu.items()?.into_iter().next() {
        let support = MenuItem::with_id(app, "support", tr(app, "Buy Me a Coffee…"), true, None::<&str>)?;
        app_menu.insert(&support, 1)?;
    }
    Ok(menu)
}
