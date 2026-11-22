// Crée un alias Finder vers un dossier, avec une icône personnalisée (sert au raccourci « Applications » du .dmg).
// Un lien symbolique ne peut pas porter d'icône personnalisée ; un alias, si.
// Usage : alias-applications <dossier cible> <chemin de l'alias> <icone.png>
import AppKit

let args = CommandLine.arguments
guard args.count == 4 else {
    fputs("Usage : alias-applications <dossier cible> <chemin de l'alias> <icone.png>\n", stderr)
    exit(2)
}
let target = URL(fileURLWithPath: args[1])
let alias = URL(fileURLWithPath: args[2])

do {
    let data = try target.bookmarkData(options: .suitableForBookmarkFile, includingResourceValuesForKeys: nil, relativeTo: nil)
    try URL.writeBookmarkData(data, to: alias)
} catch {
    fputs("Alias impossible : \(error)\n", stderr)
    exit(1)
}

guard let image = NSImage(contentsOfFile: args[3]) else {
    fputs("Icône illisible : \(args[3])\n", stderr)
    exit(1)
}
guard NSWorkspace.shared.setIcon(image, forFile: alias.path, options: []) else {
    fputs("Icône non appliquée à \(alias.path)\n", stderr)
    exit(1)
}
