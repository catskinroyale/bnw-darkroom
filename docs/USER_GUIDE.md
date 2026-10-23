# BW Darkroom user guide

BW Darkroom turns your photographs into black-and-white prints by simulating a film
darkroom. It doesn't apply a filter. It models four stages in order:

1. **Film:** how the emulsion turns colours into greys, its tone curve and its grain.
2. **Developer:** the chemistry, dilution, time, temperature and agitation.
3. **Negative:** the result of exposing and developing the film.
4. **Print:** enlarging the negative onto paper of a chosen contrast grade.

Every style in the app is a recipe of those components, so you can change one stage and
see its effect on the rest.

> **About accuracy.** Film and developer behaviour is modelled from published data
> (ISO speed, granularity, development times from the Massive Dev Chart). The curves are
> still illustrative rather than measured from each product. A style named "Tri-X ·
> D-76" means *Tri-X-like film in D-76-like developer*, not a calibrated copy.

## Contents

- [Installing](#installing) (and [update notices](#update-notices))
- [The window](#the-window)
- [Opening photos](#opening-photos)
- [Preparing the image](#preparing-the-image)
- [Choosing a style](#choosing-a-style)
- [The Develop panel](#the-develop-panel)
- [Push and pull](#push-and-pull)
- [Grain](#grain)
- [Checking detail at 100% and 200%](#checking-detail-at-100-and-200)
- [Printing controls](#printing-controls)
- [Saving and managing styles](#saving-and-managing-styles)
- [Exporting](#exporting)
- [Keyboard shortcuts](#keyboard-shortcuts)
- [Where things are stored](#where-things-are-stored)
- [Troubleshooting](#troubleshooting)

## Installing

BW Darkroom runs on Apple Silicon Macs.

1. Download `BW-Darkroom-<version>-arm64.dmg` from the
   [releases page](https://github.com/catskinroyale/bnw-darkroom/releases).
2. Open the DMG and drag **BW Darkroom** onto **Applications**.
3. The first time you open it, **right-click the app → Open**, then confirm. The app isn't
   notarised by Apple, so a normal double-click is blocked the first time. You can also
   allow it in System Settings → Privacy & Security → Open Anyway.

To update, quit the app and replace it with the new version from the DMG. Your styles
and settings are kept.

### Update notices

Once a day, when it starts, BW Darkroom asks GitHub whether a newer version has been
released. If there is one, an **Update** button appears in the top bar. Click it to see
what's new, then:

- **Download…** opens the release page in your browser, with the DMG to download.
- **Later** hides the dialog until next time.
- **Skip This Version** stops the notice for that version.

**BW Darkroom ▸ Check for Updates…** (or the button in the About window) checks
straight away. To turn off the daily check, untick **Check for updates automatically** in
the Software Update window.

The check sends nothing except the request itself, and nothing is downloaded or
installed without you.

## The window

| Area | What it's for |
|---|---|
| **Top bar** | Show or hide panels (**Folders**, **Workflow**, **Gallery**, **Images**), **Open folder…**, **Export…**, and the theme (**Auto**, **Light**, **Dark**). |
| **Folders** (left) | A tree of folders: choose the one you're working in. |
| **Image** (centre) | The selected photo rendered with the current style. |
| **Tool strip** (far right) | Icons that choose the panel beside it: **Develop** and **Styles** at the top, **Export** and **About** at the bottom. Click the active icon again to hide the panel. |
| **Develop** panel (right) | The style picker and the Image, Film, Development, Grain and Print sections. |
| **Styles** panel (right) | Every style, in groups. |
| **Styles** strip (bottom) | The current photo rendered with every style, to compare them at a glance. |
| **Images** strip (very bottom) | The photos in the chosen folder: click one to open it. |

The Develop panel's sections open and close with a click on their title, and remember how
you left them. Advanced controls sit in expandable rows inside a section (for example
**Development details**); click a row to open it.

**Sliders:** drag the track (hold **Shift** for fine control), **double-click** the
name or the knob to reset it, or **click the value** to type one (Return applies, Esc
cancels).

Panels can be resized by dragging their edges. **F** hides every panel for a clean view
of the image and brings them back.

Hover over almost any control for a short explanation.

## Opening photos

Choosing a photo takes two steps: pick a folder in the **Folders** tree on the left, then
pick a photo from the **Images** strip under the photo.

- The tree starts from **Pictures**, **Desktop**, **Home** and any memory card or
  external drive. Click a folder to show its photos in the strip; click the chevron beside
  it to show or hide its subfolders. The tree opens down to the folder you're in.
- **Browse Folder…** at the end of the tree, or **Open folder…** in the top bar, picks
  any folder, such as one on a network drive. Right-click a folder for **Show in Finder**.
- Apps and other macOS packages (`.app`, Photos and Lightroom libraries) aren't listed.
- In the strip, long file names are shortened in the middle, keeping the start and the
  frame number ("Reykjav…042.NEF"). Hover a photo to see its full name.

**Raw files first.** BW Darkroom is built for camera raw files: Nikon NEF, DNG (including
Leica) and Sony ARW. They're always listed. JPEG, TIFF and PNG files are hidden unless you
turn on **TIFF/PNG** or **JPEG** in the strip's header. The count tells you how many
files are hidden.

Use the arrow keys to step through the photos in the folder; the strip follows.

Nikon High Efficiency (HE/HE★) NEFs aren't supported; use lossless compressed NEF.

## Preparing the image

The **Image** section at the top of the Develop panel adjusts the photo *before* it
reaches the film, like choices made in the camera. These settings belong to the photo,
not to the style. They're saved next to it and come back when you reopen it.

- **Exposure** brightens or darkens the scene in stops. **Auto** picks a value from the
  image. Auto aims the average at mid-grey, so check it on night or high-key scenes.
- **Contrast** changes the scene's contrast around mid-grey, like a lighting ratio.
- **White balance** (raw only) sets which light counts as neutral. Film turns colours
  into greys, so this changes which tones come out lighter or darker, much like a
  coloured filter on the lens. Choose a preset, or set your own **Temperature** and
  **Tint** under **Temperature & highlights**.
- **Highlights** (raw only, under **Temperature & highlights**): **Reconstruct** recovers
  detail where the sensor clipped; **Clip** renders clipped areas as plain white.
- **Rotate ← / Rotate →**, and **Reset** to return to the camera's settings.

**Show original (O)** shows the photo before black-and-white conversion.

## Choosing a style

Choose a style from the **Style** row at the top of the Develop panel, or open the
**Styles** panel (its icon in the tool strip, or the grid button beside the Style row). It
lists every style in three groups: **Films & processes**, **Photographers** and **My
styles**. Click one to apply it. The **Gallery** shows the current photo in every style;
click a tile to apply that style.

Built-in styles cover:

- **Films:** Tri-X, T-Max 100 and 400, HP5, FP4, Delta 100, 400 and 3200, Double-X,
  Kentmere 100, 200 and 400.
- **Developers:** D-76 (stock, 1+1), D-96, HC-110 (dilutions B, E, H, G), Rodinal (1+25,
  1+50, 1+100), FX-55.
- **Processes and prints:** pushed and pulled processing, stand development, hard and soft
  paper grades.

### Photographer presets

The **Photographers** group offers looks inspired by Don McCullin, Ansel Adams, Bill
Brandt, Josef Koudelka and Edward Weston. Hover one to see the look and what is
documented about the photographer's working methods versus what is interpretation.
Clicking one uses its film, developer, processing and print recipe, and sets scene
contrast where the look calls for it.

## The Develop panel

Below the style picker, the Develop panel has these sections:

| Section | Controls | Expandable rows |
|---|---|---|
| **Image** | Auto, rotate, reset; exposure, contrast; white balance presets | **Temperature & highlights** |
| **Film** | Film stock, format, enlargement | |
| **Development** | Developer, dilution, push/pull, and a readout of the time | **Development details** (EI, exposure compensation, time, temperature, agitation) and **Characteristic curve** |
| **Grain** | Size, amount, roughness; Film's grain, Randomise, New pattern | |
| **Print** | Paper grade, print exposure | |

Changes show in the image straight away. They change a *draft* of the selected style; see
[Saving and managing styles](#saving-and-managing-styles) to keep them.

**Development time** follows the published time for the film, developer, dilution and EI
(from the Massive Dev Chart, whole stops only). The readout says whether the time is
**published**, **interpolated** or **estimated**. Under **Development details**, choose
**Your own** to set the time yourself. Longer development raises contrast and grain.

**Agitation** (under **Development details**): continuous, every 30 s or 60 s,
semi-stand or stand. Less agitation gives more edge sharpness and holds back the
highlights (compensation). Stand needs a long time.

## Push and pull

Your photo is already correctly exposed, so in BW Darkroom push and pull change
**development only**. The film is exposed at box speed and developed for the time
published for the pushed or pulled EI. That's the Zone System's N+ and N− development.

- **Push** keeps blacks black, opens the shadows slightly, brightens the highlights and
  adds grain. Contrast rises.
- **Pull** does the reverse: softer contrast, greyer highlights.

The effect depends on the published times. For example, Tri-X in D-76 at Push 1 uses the
same time as box speed, because Kodak treats one stop as within the film's latitude.

For the classic **underexposed push** (empty, blocked shadows), push and also lower
**Exposure comp.** under **Development details**.

The print keeps the same black point whatever the development, as a darkroom printer
does when working to "minimum time for maximum black". So development changes show as
they would under a real enlarger.

## Grain

Grain comes from the film and developer, and shows more the more the negative is
enlarged. In the **Film** section:

- **Format** sets the film size: 35 mm shows the most grain at a given print size; 4×5
  and 8×10 almost none.
- **Enlarge** uses only the centre of the frame, which means enlarging the negative more.

In the **Grain** section, the readout **Grain at print** rates the result from *very
fine* to *very coarse*, with Kodak's RMS granularity figure.

You can also adjust the grain beyond what the film gives:

| Control | Effect |
|---|---|
| **Size** | Coarseness of the grain clumps. Strength stays the same. |
| **Amount** | Strength of the grain. 0 removes it. |
| **Roughness** | Below 0: softer and smoother. Above 0: grittier, with more isolated dark and bright specks. |
| **Randomise** | A new pattern with random size, amount and roughness within a usable range. |
| **Film's grain** | Back to the film's own grain (size ×1, amount ×1, roughness 0). |
| **New pattern** | The same grain, a different random pattern. |

The defaults are exactly the film's own grain. When you change them, the readout says
**adjusted from the film**.

## Checking detail at 100% and 200%

Grain and edge effects only show properly at full resolution. Press **D** (or tick
**Detail 1:1**) and click the image to choose the area. An orange outline shows what the
Detail window displays.

- **100%** shows one image pixel per screen pixel: the export at full size, with
  exactly the grain the export will have.
- **200%** magnifies two times, for judging grain structure.

The Detail window updates first whenever you change a setting, so you can fine-tune
development and grain while watching it. A small spinner shows while it catches up.

To move the area you're inspecting, **drag inside the Detail window**, as if moving the
photo under a loupe, or click the main image. Drag the window by its title bar to put it
wherever it's most useful.

## Printing controls

In the **Print** section:

- **Paper grade** (0–5) sets print contrast. Grade 2 is normal; higher is harder.
- **Print exposure** makes the whole print darker (more exposure) or lighter (less), in
  stops, relative to the automatic exposure.

The print is shown as paper reflectance, so the deepest black is the paper's black. On
screen that's very dark grey rather than pure black, as it would look on a glossy
print.

## Saving and managing styles

When you change a style, its name shows *(edited)* and the buttons under **Develop**
offer:

- **Save:** update your own style (built-in styles are read-only).
- **Save as…:** keep the changes as a new style of your own.
- **Revert:** discard the changes.

Each row in the Styles panel has a **…** menu when you hover it, and the same menu opens
when you right-click a row or a Gallery tile:

| Item | What it does |
|---|---|
| **Edit…** | Applies the style and opens every section of the Develop panel. |
| **Duplicate** | Makes an editable copy. |
| **Rename…** | Renames one of your styles. |
| **Export…** | Saves the style as a `.json` file to share or back up. |
| **Move up / Move down** | Reorders your own styles, below the built-ins. |
| **Delete…** | Removes one of your styles, after asking. |

**Import…** at the top of the Styles panel adds styles from exported `.json` files.
Styles with a clashing name are renamed.

## Exporting

**Export…** (or **⌘E**) renders the selected photo at full resolution with the current
style.

| Option | Choices |
|---|---|
| **Format** | **Same as original** (JPEG stays JPEG, PNG stays PNG, raw and TIFF become TIFF), **TIFF 16-bit**, **JPEG**, **PNG 16-bit**. |
| **JPEG quality** | 1–100. 85–95 suits most prints; below 70 grain turns blocky. |
| **Size** | **Full** resolution, or a long edge of 4096, 2048 or 1080 pixels, or a custom size. |
| **File name** | A template built from tokens (below). The extension is added for you. |
| **Numbering** | The `{n}` counter: the next number and how many digits. |
| **If file exists** | **Add -2, -3…** or **Overwrite**. |
| **Folder** | A `bw-exports` folder next to each image (the default), or a folder you choose. |
| **Recipe** | Also save a `.json` recipe of the full workflow, so the print can be reproduced. |

File-name tokens:

| Token | Becomes |
|---|---|
| `{name}` | Original file name without its extension |
| `{style}` | Style name |
| `{film}` | Film |
| `{developer}` | Developer and dilution |
| `{ei}` | EI the film was developed for |
| `{date}` | Export date, YYYY-MM-DD (UTC) |
| `{time}` | Export time, HHMMSS (UTC) |
| `{n}` | Sequence number |

The default template is `{name}__{style}`.

**Metadata.** Exports keep the photo's EXIF record: camera, lens, exposure, dates, GPS,
artist and copyright. The orientation is reset because rotation is already applied, the
software is recorded as BW Darkroom, and the development recipe goes in the comment
field.

## Keyboard shortcuts

| Key | Action |
|---|---|
| **← → ↑ ↓** | Previous / next photo |
| **O** | Show the original |
| **D** | Detail window on or off |
| **B** | Show or hide the folder tree |
| **W** | Show or hide the Develop / Styles panel |
| **G** | Show or hide the style gallery |
| **I** | Show or hide the image strip |
| **F** | Focus: hide or show all panels |
| **⌘E** | Export |

## Where things are stored

| What | Where |
|---|---|
| Your styles | `~/Library/Application Support/bw-darkroom/styles.json` |
| Per-photo adjustments | A `<photo>.bw.json` file next to each photo you've adjusted |
| Exports | A `bw-exports` folder next to the photo, unless you chose another folder |

Nothing is written into your original files.

## Troubleshooting

**The app won't open the first time.** Right-click it → Open, or allow it in System
Settings → Privacy & Security.

**A raw file won't open.** Check it isn't a Nikon High Efficiency NEF; those aren't
supported.

**A night photo looks washed out after Auto exposure.** Auto aims the average at
mid-grey, which overexposes mostly dark scenes. Set **Exposure** by hand.

**My JPEGs don't appear.** Turn on **JPEG** (or **TIFF/PNG**) in the image strip's header.

**Licences and version.** BW Darkroom ▸ About BW Darkroom shows the version, the
project's licences and every third-party component.
