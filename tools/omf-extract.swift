// omf-extract : lit le texte d'un PDF ou d'une image (OCR) avec les composants natifs de macOS.
//   PDFKit pour le texte d'un PDF ; Vision pour reconnaître le texte d'une image ou d'un PDF scanné
//   (français + anglais). Le texte est écrit sur la sortie standard. Rien ne quitte le Mac.
// Usage : omf-extract <fichier>      Code de sortie : 0 = lu (le texte peut être vide), 2 = illisible.
import CoreGraphics
import Foundation
import ImageIO
import PDFKit
import Vision

let maxPagesForOCR = 5
let maxOutputCharacters = 1_000_000
let minimumPDFTextLength = 1

func recognizeText(in image: CGImage) -> String {
    let request = VNRecognizeTextRequest()
    request.recognitionLevel = .accurate
    request.usesLanguageCorrection = true
    request.recognitionLanguages = ["fr-FR", "en-US"]
    let handler = VNImageRequestHandler(cgImage: image, options: [:])
    do { try handler.perform([request]) } catch { return "" }
    return (request.results ?? []).compactMap { $0.topCandidates(1).first?.string }.joined(separator: "\n")
}

func loadImage(_ url: URL) -> CGImage? {
    guard let source = CGImageSourceCreateWithURL(url as CFURL, nil) else { return nil }
    return CGImageSourceCreateImageAtIndex(source, 0, nil)
}

/// Dessine une page de PDF sur fond blanc, en agrandissant les petites pages pour l'OCR.
func render(_ page: PDFPage) -> CGImage? {
    let box = page.bounds(for: .mediaBox)
    guard box.width > 0, box.height > 0 else { return nil }
    let scale = min(2.0, 4000.0 / max(box.width, box.height))
    let width = Int(box.width * scale), height = Int(box.height * scale)
    guard let context = CGContext(
        data: nil, width: width, height: height, bitsPerComponent: 8, bytesPerRow: 0,
        space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
    ) else { return nil }
    context.setFillColor(CGColor(red: 1, green: 1, blue: 1, alpha: 1))
    context.fill(CGRect(x: 0, y: 0, width: width, height: height))
    context.scaleBy(x: scale, y: scale)
    page.draw(with: .mediaBox, to: context)
    return context.makeImage()
}

func extract(_ url: URL) -> String? {
    if url.pathExtension.lowercased() == "pdf" {
        guard let document = PDFDocument(url: url), !document.isLocked else { return nil }
        let text = document.string ?? ""
        if text.trimmingCharacters(in: .whitespacesAndNewlines).count >= minimumPDFTextLength { return text }
        // PDF scanné : pas de texte, on lit les premières pages comme des images.
        var pages: [String] = []
        for index in 0..<min(document.pageCount, maxPagesForOCR) {
            if let page = document.page(at: index), let image = render(page) {
                pages.append(recognizeText(in: image))
            }
        }
        return pages.joined(separator: "\n")
    }
    guard let image = loadImage(url) else { return nil }
    return recognizeText(in: image)
}

guard CommandLine.arguments.count == 2 else {
    FileHandle.standardError.write(Data("usage: omf-extract <fichier>\n".utf8))
    exit(2)
}
guard let text = extract(URL(fileURLWithPath: CommandLine.arguments[1])) else { exit(2) }
FileHandle.standardOutput.write(Data(String(text.prefix(maxOutputCharacters)).utf8))
exit(0)
