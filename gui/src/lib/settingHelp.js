// One-line explanations shown in each setting's ⓘ tooltip, keyed by locale
// and setting id. Wording follows the DJI Mic 3 user manual (v1.0) where it
// describes the feature; ids missing here simply get no ⓘ. Falls back to
// English for a missing translation.

const en = {
  "noise-cancel-power": "Reduces background noise picked up by the transmitters.",
  "noise-cancel": "Basic keeps a more natural voice; Strong removes more noise, for loud places.",
  "noise-cancel-button":
    "When on, the transmitter's power button turns noise cancellation on or off (a double press on the Mic 3).",
  "low-cut": "Cuts frequencies of 100 Hz and below to reduce low-frequency noise such as wind and rumble.",
  stereo:
    "Mono mixes every transmitter into both channels. Stereo puts TX1 left and TX2 right. 4CH gives each of up to four transmitters its own track (only some PC software and the DJI camera adapter support it). The Mic 3 has no Safety Track mode — DJI replaced it with 32-bit float internal recording.",
  "safety-track":
    "Records a backup copy 6 dB quieter on the second channel, so you have a clean take if the main one clips.",
  "clip-limiter": "Limits the output so loud sounds don't distort.",
  "rx-gain": "Receiver output gain. If the sound is too loud or distorted, turn this down first.",
  lossless: "Transmitters send uncompressed 48 kHz / 24-bit audio to the receiver for the best quality.",
  mute: "Silences this transmitter's audio without turning it off.",
  "monitor-gain": "Volume of the headphone output on the receiver.",
  "tx-clip-control":
    "Adaptive gain control: smooths sudden volume spikes to prevent clipping (the manual's “Automatic” mode).",
  "loudness-balance":
    "Adaptive gain control: lifts quiet input and tames loud input to keep levels even (the manual's “Dynamic” mode).",
  "auto-off-15m": "The receiver powers off if no transmitter links to it within 15 minutes of powering on.",
  "tx-auto-off-15m":
    "A transmitter that isn't connected to anything and isn't recording powers off after 15 minutes.",
  "camera-power":
    "The receiver turns on and off with the camera when connected by the 3.5 mm cable or the camera adapter.",
  "plug-free":
    "With the receiver connected to a phone, lets the phone keep playing sound through its own speaker instead of going silent.",
  "mic-leds": "Turns the transmitters' status lights on or off.",
  vibration: "Transmitters vibrate to confirm actions such as starting or stopping recording.",
  float32:
    "Transmitters record internally in 32-bit float, which leaves far more headroom for editing. Recording time drops from about 57 h to 43 h.",
  "loop-recording":
    "Splits internal recordings into segments and overwrites the oldest files when storage is full.",
  "startup-recording": "Transmitters start internal recording as soon as they're powered on.",
  "low-power-recording":
    "Transmitters start internal recording automatically when the receiver or the connected camera/phone runs low on battery.",
  "rec-stop": "Pressing a transmitter's power button starts or stops its internal recording.",
  record: "Starts or stops internal recording on this transmitter.",
  "voice-tone": "Standard, Rich (fuller low end) or Bright (crisper highs).",
  "tx-gain": "Transmitter input gain. Turn it down if the level bar goes red.",
};

const es = {
  "noise-cancel-power": "Reduce el ruido de fondo que captan los transmisores.",
  "noise-cancel": "Básica conserva una voz más natural; Fuerte elimina más ruido, para sitios ruidosos.",
  "noise-cancel-button":
    "Si está activado, el botón de encendido del transmisor activa o desactiva la cancelación de ruido (doble pulsación en el Mic 3).",
  "low-cut": "Corta las frecuencias de 100 Hz o menos para reducir ruidos graves como el viento o los zumbidos.",
  stereo:
    "Mono mezcla todos los transmisores en ambos canales. Estéreo pone el TX1 a la izquierda y el TX2 a la derecha. 4CH da a cada uno de hasta cuatro transmisores su propia pista (solo algunos programas de ordenador y el adaptador de cámara de DJI lo admiten). El Mic 3 no tiene modo de pista de seguridad: DJI lo sustituyó por la grabación interna en 32 bits flotante.",
  "safety-track":
    "Graba una copia de seguridad 6 dB más baja en el segundo canal, para tener una toma limpia si la principal satura.",
  "clip-limiter": "Limita la salida para que los sonidos fuertes no distorsionen.",
  "rx-gain": "Ganancia de salida del receptor. Si el sonido está demasiado alto o distorsiona, bájala primero.",
  lossless: "Los transmisores envían al receptor audio sin comprimir de 48 kHz / 24 bits para la máxima calidad.",
  mute: "Silencia el audio de este transmisor sin apagarlo.",
  "monitor-gain": "Volumen de la salida de auriculares del receptor.",
  "tx-clip-control":
    "Control de ganancia adaptativo: suaviza los picos repentinos de volumen para evitar la saturación (el modo «Automático» del manual).",
  "loudness-balance":
    "Control de ganancia adaptativo: sube las entradas débiles y contiene las fuertes para mantener un nivel uniforme (el modo «Dinámico» del manual).",
  "auto-off-15m": "El receptor se apaga si ningún transmisor se vincula en los 15 minutos siguientes a encenderlo.",
  "tx-auto-off-15m":
    "Un transmisor que no está conectado a nada y no está grabando se apaga a los 15 minutos.",
  "camera-power":
    "El receptor se enciende y se apaga con la cámara cuando está conectado por el cable de 3,5 mm o el adaptador de cámara.",
  "plug-free":
    "Con el receptor conectado a un móvil, el móvil sigue reproduciendo el sonido por su propio altavoz en lugar de quedarse mudo.",
  "mic-leds": "Enciende o apaga las luces de estado de los transmisores.",
  vibration: "Los transmisores vibran para confirmar acciones como empezar o detener una grabación.",
  float32:
    "Los transmisores graban internamente en 32 bits flotante, que deja mucho más margen para editar. El tiempo de grabación baja de unas 57 h a 43 h.",
  "loop-recording":
    "Divide las grabaciones internas en fragmentos y sobrescribe los archivos más antiguos cuando la memoria se llena.",
  "startup-recording": "Los transmisores empiezan a grabar internamente nada más encenderse.",
  "low-power-recording":
    "Los transmisores empiezan a grabar automáticamente cuando al receptor o a la cámara/móvil conectados les queda poca batería.",
  "rec-stop": "Al pulsar el botón de encendido de un transmisor, empieza o detiene su grabación interna.",
  record: "Empieza o detiene la grabación interna de este transmisor.",
  "voice-tone": "Estándar, Rico (graves más llenos) o Brillante (agudos más nítidos).",
  "tx-gain": "Ganancia de entrada del transmisor. Bájala si la barra de nivel se pone roja.",
};

const fr = {
  "noise-cancel-power": "Réduit le bruit de fond capté par les émetteurs.",
  "noise-cancel": "Basique garde une voix plus naturelle ; Forte supprime davantage de bruit, pour les lieux bruyants.",
  "noise-cancel-button":
    "Activé, le bouton d'alimentation de l'émetteur active ou désactive la réduction de bruit (double appui sur le Mic 3).",
  "low-cut": "Coupe les fréquences de 100 Hz et moins pour réduire les bruits graves comme le vent ou les grondements.",
  stereo:
    "Mono mélange tous les émetteurs sur les deux canaux. Stéréo place le TX1 à gauche et le TX2 à droite. 4CH donne à chacun des quatre émetteurs maximum sa propre piste (seuls certains logiciels et l'adaptateur caméra DJI le prennent en charge). Le Mic 3 n'a pas de mode Piste de sécurité : DJI l'a remplacé par l'enregistrement interne en 32 bits flottant.",
  "safety-track":
    "Enregistre une copie de secours 6 dB plus basse sur le second canal, pour avoir une prise propre si la principale sature.",
  "clip-limiter": "Limite la sortie pour que les sons forts ne saturent pas.",
  "rx-gain": "Gain de sortie du récepteur. Si le son est trop fort ou sature, baissez-le d'abord.",
  lossless: "Les émetteurs envoient au récepteur un audio non compressé 48 kHz / 24 bits pour une qualité maximale.",
  mute: "Coupe le son de cet émetteur sans l'éteindre.",
  "monitor-gain": "Volume de la sortie casque du récepteur.",
  "tx-clip-control":
    "Contrôle de gain adaptatif : lisse les pics de volume soudains pour éviter l'écrêtage (le mode « Automatique » du manuel).",
  "loudness-balance":
    "Contrôle de gain adaptatif : relève les entrées faibles et atténue les fortes pour garder un niveau régulier (le mode « Dynamique » du manuel).",
  "auto-off-15m": "Le récepteur s'éteint si aucun émetteur ne s'y connecte dans les 15 minutes suivant l'allumage.",
  "tx-auto-off-15m":
    "Un émetteur qui n'est connecté à rien et n'enregistre pas s'éteint au bout de 15 minutes.",
  "camera-power":
    "Le récepteur s'allume et s'éteint avec la caméra lorsqu'il y est relié par le câble 3,5 mm ou l'adaptateur caméra.",
  "plug-free":
    "Avec le récepteur branché sur un téléphone, le téléphone continue de diffuser le son par son propre haut-parleur au lieu de devenir muet.",
  "mic-leds": "Allume ou éteint les voyants d'état des émetteurs.",
  vibration: "Les émetteurs vibrent pour confirmer des actions comme le début ou l'arrêt d'un enregistrement.",
  float32:
    "Les émetteurs enregistrent en interne en 32 bits flottant, ce qui laisse bien plus de marge au montage. La durée d'enregistrement passe d'environ 57 h à 43 h.",
  "loop-recording":
    "Découpe les enregistrements internes en segments et écrase les fichiers les plus anciens quand la mémoire est pleine.",
  "startup-recording": "Les émetteurs démarrent l'enregistrement interne dès leur mise sous tension.",
  "low-power-recording":
    "Les émetteurs démarrent l'enregistrement interne automatiquement quand le récepteur ou la caméra/le téléphone connecté a une batterie faible.",
  "rec-stop": "Appuyer sur le bouton d'alimentation d'un émetteur démarre ou arrête son enregistrement interne.",
  record: "Démarre ou arrête l'enregistrement interne de cet émetteur.",
  "voice-tone": "Standard, Riche (graves plus pleins) ou Brillant (aigus plus nets).",
  "tx-gain": "Gain d'entrée de l'émetteur. Baissez-le si la barre de niveau passe au rouge.",
};

const it = {
  "noise-cancel-power": "Riduce il rumore di fondo captato dai trasmettitori.",
  "noise-cancel": "Base mantiene una voce più naturale; Forte elimina più rumore, per i luoghi rumorosi.",
  "noise-cancel-button":
    "Se attivo, il pulsante di accensione del trasmettitore attiva o disattiva la cancellazione del rumore (doppia pressione sul Mic 3).",
  "low-cut": "Taglia le frequenze di 100 Hz e inferiori per ridurre i rumori bassi come vento e rimbombi.",
  stereo:
    "Mono mixa tutti i trasmettitori su entrambi i canali. Stereo mette il TX1 a sinistra e il TX2 a destra. 4CH dà a ciascuno dei quattro trasmettitori al massimo una traccia propria (solo alcuni software per PC e l'adattatore per fotocamera DJI lo supportano). Il Mic 3 non ha la modalità Traccia di sicurezza: DJI l'ha sostituita con la registrazione interna a 32 bit float.",
  "safety-track":
    "Registra una copia di sicurezza 6 dB più bassa sul secondo canale, così hai una ripresa pulita se la principale satura.",
  "clip-limiter": "Limita l'uscita in modo che i suoni forti non distorcano.",
  "rx-gain": "Guadagno di uscita del ricevitore. Se il suono è troppo forte o distorce, abbassalo per primo.",
  lossless: "I trasmettitori inviano al ricevitore audio non compresso a 48 kHz / 24 bit per la massima qualità.",
  mute: "Silenzia l'audio di questo trasmettitore senza spegnerlo.",
  "monitor-gain": "Volume dell'uscita cuffie del ricevitore.",
  "tx-clip-control":
    "Controllo adattivo del guadagno: smussa i picchi di volume improvvisi per evitare il clipping (la modalità «Automatica» del manuale).",
  "loudness-balance":
    "Controllo adattivo del guadagno: alza gli ingressi deboli e contiene quelli forti per mantenere un livello uniforme (la modalità «Dinamica» del manuale).",
  "auto-off-15m": "Il ricevitore si spegne se nessun trasmettitore si collega entro 15 minuti dall'accensione.",
  "tx-auto-off-15m":
    "Un trasmettitore non collegato a nulla e che non sta registrando si spegne dopo 15 minuti.",
  "camera-power":
    "Il ricevitore si accende e si spegne con la fotocamera quando è collegato con il cavo da 3,5 mm o con l'adattatore per fotocamera.",
  "plug-free":
    "Con il ricevitore collegato a un telefono, il telefono continua a riprodurre l'audio dal proprio altoparlante invece di restare muto.",
  "mic-leds": "Accende o spegne le luci di stato dei trasmettitori.",
  vibration: "I trasmettitori vibrano per confermare azioni come l'avvio o l'arresto di una registrazione.",
  float32:
    "I trasmettitori registrano internamente a 32 bit float, che lascia molto più margine in post-produzione. Il tempo di registrazione scende da circa 57 h a 43 h.",
  "loop-recording":
    "Divide le registrazioni interne in segmenti e sovrascrive i file più vecchi quando la memoria è piena.",
  "startup-recording": "I trasmettitori avviano la registrazione interna appena vengono accesi.",
  "low-power-recording":
    "I trasmettitori avviano automaticamente la registrazione interna quando il ricevitore o la fotocamera/il telefono collegati hanno la batteria scarica.",
  "rec-stop": "Premendo il pulsante di accensione di un trasmettitore si avvia o si ferma la sua registrazione interna.",
  record: "Avvia o ferma la registrazione interna di questo trasmettitore.",
  "voice-tone": "Standard, Ricco (bassi più pieni) o Brillante (alti più nitidi).",
  "tx-gain": "Guadagno di ingresso del trasmettitore. Abbassalo se la barra del livello diventa rossa.",
};

const de = {
  "noise-cancel-power": "Verringert die Hintergrundgeräusche, die die Sender aufnehmen.",
  "noise-cancel": "Basis klingt natürlicher; Stark entfernt mehr Rauschen, für laute Umgebungen.",
  "noise-cancel-button":
    "Wenn aktiv, schaltet die Ein/Aus-Taste des Senders die Rauschunterdrückung ein oder aus (Doppeldruck beim Mic 3).",
  "low-cut": "Schneidet Frequenzen ab 100 Hz abwärts ab, um tiefe Störgeräusche wie Wind und Rumpeln zu verringern.",
  stereo:
    "Mono mischt alle Sender auf beide Kanäle. Stereo legt TX1 links und TX2 rechts ab. 4CH gibt jedem von bis zu vier Sendern eine eigene Spur (nur manche PC-Software und der DJI-Kameraadapter unterstützen das). Das Mic 3 hat keinen Sicherheitsspur-Modus – DJI hat ihn durch die interne 32-Bit-Float-Aufnahme ersetzt.",
  "safety-track":
    "Nimmt auf dem zweiten Kanal eine 6 dB leisere Sicherungskopie auf, damit du eine saubere Aufnahme hast, falls die Hauptspur übersteuert.",
  "clip-limiter": "Begrenzt den Ausgang, damit laute Geräusche nicht verzerren.",
  "rx-gain": "Ausgangsverstärkung des Empfängers. Wenn der Ton zu laut ist oder verzerrt, zuerst diese senken.",
  lossless: "Die Sender übertragen unkomprimiertes 48-kHz-/24-Bit-Audio an den Empfänger, für beste Qualität.",
  mute: "Schaltet den Ton dieses Senders stumm, ohne ihn auszuschalten.",
  "monitor-gain": "Lautstärke des Kopfhörerausgangs am Empfänger.",
  "tx-clip-control":
    "Adaptive Verstärkungsregelung: glättet plötzliche Lautstärkespitzen, um Übersteuerung zu vermeiden (der Modus „Automatisch“ im Handbuch).",
  "loudness-balance":
    "Adaptive Verstärkungsregelung: hebt leise Signale an und dämpft laute, für einen gleichmäßigen Pegel (der Modus „Dynamisch“ im Handbuch).",
  "auto-off-15m": "Der Empfänger schaltet sich aus, wenn sich innerhalb von 15 Minuten nach dem Einschalten kein Sender verbindet.",
  "tx-auto-off-15m":
    "Ein Sender, der mit nichts verbunden ist und nicht aufnimmt, schaltet sich nach 15 Minuten aus.",
  "camera-power":
    "Der Empfänger schaltet sich mit der Kamera ein und aus, wenn er über das 3,5-mm-Kabel oder den Kameraadapter verbunden ist.",
  "plug-free":
    "Wenn der Empfänger an einem Smartphone hängt, spielt das Smartphone den Ton weiter über den eigenen Lautsprecher ab, statt stumm zu werden.",
  "mic-leds": "Schaltet die Statusleuchten der Sender ein oder aus.",
  vibration: "Die Sender vibrieren, um Aktionen wie Aufnahmestart oder -stopp zu bestätigen.",
  float32:
    "Die Sender nehmen intern in 32-Bit-Float auf, was beim Schnitt deutlich mehr Spielraum lässt. Die Aufnahmezeit sinkt von etwa 57 h auf 43 h.",
  "loop-recording":
    "Teilt interne Aufnahmen in Abschnitte und überschreibt die ältesten Dateien, wenn der Speicher voll ist.",
  "startup-recording": "Die Sender starten die interne Aufnahme, sobald sie eingeschaltet werden.",
  "low-power-recording":
    "Die Sender starten die interne Aufnahme automatisch, wenn der Akku des Empfängers oder der verbundenen Kamera/des Smartphones schwach wird.",
  "rec-stop": "Ein Druck auf die Ein/Aus-Taste eines Senders startet oder stoppt dessen interne Aufnahme.",
  record: "Startet oder stoppt die interne Aufnahme dieses Senders.",
  "voice-tone": "Standard, Voll (kräftigere Tiefen) oder Hell (klarere Höhen).",
  "tx-gain": "Eingangsverstärkung des Senders. Senken, wenn die Pegelanzeige rot wird.",
};

const nl = {
  "noise-cancel-power": "Vermindert het achtergrondgeluid dat de zenders oppikken.",
  "noise-cancel": "Basis houdt een natuurlijkere stem; Sterk verwijdert meer ruis, voor lawaaierige plekken.",
  "noise-cancel-button":
    "Als dit aan staat, zet de aan/uit-knop van de zender ruisonderdrukking aan of uit (dubbel drukken op de Mic 3).",
  "low-cut": "Snijdt frequenties van 100 Hz en lager af om laag geluid zoals wind en gerommel te verminderen.",
  stereo:
    "Mono mengt alle zenders op beide kanalen. Stereo zet TX1 links en TX2 rechts. 4CH geeft elk van maximaal vier zenders een eigen spoor (alleen sommige pc-software en de DJI-camera-adapter ondersteunen dit). De Mic 3 heeft geen veiligheidsspoor-modus: DJI heeft die vervangen door interne opname in 32-bit float.",
  "safety-track":
    "Neemt op het tweede kanaal een reservekopie op die 6 dB zachter is, zodat je een schone opname hebt als de hoofdopname overstuurt.",
  "clip-limiter": "Begrenst de uitgang zodat harde geluiden niet vervormen.",
  "rx-gain": "Uitgangsversterking van de ontvanger. Is het geluid te hard of vervormd, zet dit dan eerst lager.",
  lossless: "De zenders sturen ongecomprimeerde 48 kHz / 24-bit audio naar de ontvanger, voor de beste kwaliteit.",
  mute: "Dempt het geluid van deze zender zonder hem uit te zetten.",
  "monitor-gain": "Volume van de koptelefoonuitgang op de ontvanger.",
  "tx-clip-control":
    "Adaptieve versterkingsregeling: vlakt plotselinge volumepieken af om clipping te voorkomen (de modus ‘Automatisch’ in de handleiding).",
  "loudness-balance":
    "Adaptieve versterkingsregeling: versterkt zachte signalen en tempert harde, voor een gelijkmatig niveau (de modus ‘Dynamisch’ in de handleiding).",
  "auto-off-15m": "De ontvanger gaat uit als er binnen 15 minuten na het aanzetten geen zender verbinding maakt.",
  "tx-auto-off-15m":
    "Een zender die nergens mee verbonden is en niet opneemt, gaat na 15 minuten uit.",
  "camera-power":
    "De ontvanger gaat aan en uit met de camera als hij via de 3,5mm-kabel of de camera-adapter is aangesloten.",
  "plug-free":
    "Met de ontvanger aangesloten op een telefoon blijft de telefoon geluid afspelen via de eigen luidspreker, in plaats van stil te vallen.",
  "mic-leds": "Zet de statuslampjes van de zenders aan of uit.",
  vibration: "De zenders trillen om acties zoals het starten of stoppen van een opname te bevestigen.",
  float32:
    "De zenders nemen intern op in 32-bit float, wat veel meer ruimte geeft bij het bewerken. De opnametijd daalt van ongeveer 57 u naar 43 u.",
  "loop-recording":
    "Splitst interne opnames in stukken en overschrijft de oudste bestanden als het geheugen vol is.",
  "startup-recording": "De zenders starten de interne opname zodra ze worden aangezet.",
  "low-power-recording":
    "De zenders starten automatisch de interne opname als de batterij van de ontvanger of de aangesloten camera/telefoon bijna leeg is.",
  "rec-stop": "Op de aan/uit-knop van een zender drukken start of stopt de interne opname ervan.",
  record: "Start of stopt de interne opname van deze zender.",
  "voice-tone": "Standaard, Vol (vollere lage tonen) of Helder (scherpere hoge tonen).",
  "tx-gain": "Ingangsversterking van de zender. Zet hem lager als de niveaubalk rood wordt.",
};


// Settings added with multi-model support.
Object.assign(en, {
  "auto-record": "Transmitters start internal recording on their own: as soon as they power on, or whenever they link to the receiver.",
  "dual-file": "Keeps the original recording alongside a second file with the noise cancelling and other processing applied."
});
Object.assign(es, {
  "auto-record": "Los transmisores empiezan a grabar internamente solos: nada más encenderse o cada vez que se conectan al receptor.",
  "dual-file": "Guarda la grabación original junto a un segundo archivo con la cancelación de ruido y el resto del procesado aplicados."
});
Object.assign(fr, {
  "auto-record": "Les émetteurs lancent l'enregistrement interne d'eux-mêmes : dès l'allumage, ou dès qu'ils se connectent au récepteur.",
  "dual-file": "Conserve l'enregistrement original avec un second fichier où la réduction de bruit et les autres traitements sont appliqués."
});
Object.assign(it, {
  "auto-record": "I trasmettitori avviano da soli la registrazione interna: appena accesi, oppure ogni volta che si collegano al ricevitore.",
  "dual-file": "Mantiene la registrazione originale insieme a un secondo file con la cancellazione del rumore e le altre elaborazioni applicate."
});
Object.assign(de, {
  "auto-record": "Die Sender starten die interne Aufnahme selbst: sobald sie eingeschaltet werden oder sich mit dem Empfänger verbinden.",
  "dual-file": "Behält die Originalaufnahme und speichert eine zweite Datei mit Rauschunterdrückung und übriger Bearbeitung."
});
Object.assign(nl, {
  "auto-record": "De zenders starten zelf de interne opname: zodra ze aangaan, of telkens wanneer ze verbinding maken met de ontvanger.",
  "dual-file": "Bewaart de originele opname naast een tweede bestand met ruisonderdrukking en de andere bewerkingen toegepast."
});

const HELP = { en, "es-ES": es, fr, it, de, nl };

/** The ⓘ explanation for `id` in `locale`, falling back to English. */
export function helpFor(locale, id) {
  return HELP[locale]?.[id] ?? en[id];
}
