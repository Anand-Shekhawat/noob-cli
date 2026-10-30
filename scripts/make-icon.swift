// Draws Noob's icon: "noob>" and a cursor on a rounded teal square.
// Run on a Mac:  swift scripts/make-icon.swift assets/noob-icon.png
import AppKit

let size: CGFloat = 1024
let output = CommandLine.arguments.dropFirst().first ?? "noob-icon.png"

let image = NSImage(size: NSSize(width: size, height: size))
image.lockFocus()

// Apple's icon grid leaves a margin around the rounded square.
let square = NSRect(x: 100, y: 100, width: size - 200, height: size - 200)
let shape = NSBezierPath(roundedRect: square, xRadius: 185, yRadius: 185)
let teal = NSGradient(
    starting: NSColor(calibratedRed: 0.10, green: 0.62, blue: 0.62, alpha: 1),
    ending: NSColor(calibratedRed: 0.05, green: 0.33, blue: 0.42, alpha: 1)
)!
teal.draw(in: shape, angle: -90)

let font = NSFont.monospacedSystemFont(ofSize: 170, weight: .bold)
let text = NSAttributedString(string: "noob>", attributes: [
    .font: font,
    .foregroundColor: NSColor.white,
])
let textSize = text.size()
let textOrigin = NSPoint(x: (size - textSize.width) / 2 - 30, y: (size - textSize.height) / 2 + 10)
text.draw(at: textOrigin)

// The cursor bar after the prompt.
let cursor = NSRect(x: textOrigin.x + textSize.width + 22, y: textOrigin.y + 30, width: 34, height: textSize.height - 60)
NSColor(calibratedRed: 0.98, green: 0.82, blue: 0.35, alpha: 1).setFill()
NSBezierPath(roundedRect: cursor, xRadius: 8, yRadius: 8).fill()

image.unlockFocus()

guard let tiff = image.tiffRepresentation,
      let bitmap = NSBitmapImageRep(data: tiff),
      let png = bitmap.representation(using: .png, properties: [:]) else {
    fatalError("Couldn't make the PNG")
}
try! png.write(to: URL(fileURLWithPath: output))
print("Wrote \(output)")
