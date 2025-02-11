//
//  Array Extensions.swift
//  TramTrackerSwiftUI
//
//  Created on 17/3/2024.
//

import Foundation

extension Array {
    func safeElement(at index: Index) -> Element? {
        return indices.contains(index) ? self[index] : nil
    }
}
