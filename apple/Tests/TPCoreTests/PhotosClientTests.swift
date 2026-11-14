import Foundation
import Testing
@testable import TPCore

extension TPCoreTestsRoot {

@Suite struct PhotosClientTests {
    private func frame(_ id: Int, _ bytes: [UInt8]) -> [UInt8] {
        func be(_ value: Int) -> [UInt8] { [24, 16, 8, 0].map { UInt8((value >> $0) & 0xFF) } }
        return be(id) + be(bytes.count) + bytes
    }

    @Test func unpacksABulkThumbnailResponse() {
        let data = Data(frame(7, [0xFF, 0xD8, 0x01]) + frame(9, []) + frame(300_000, [0xAA]))
        let result = PhotosClient.unpackThumbnails(data)

        #expect(result.count == 2)
        #expect(result[7] == Data([0xFF, 0xD8, 0x01]))
        #expect(result[9] == nil)
        #expect(result[300_000] == Data([0xAA]))
    }

    @Test func stopsAtATruncatedFrame() {
        let data = Data(frame(1, [1, 2]) + [0, 0, 0, 2, 0, 0, 0, 9, 1])
        #expect(PhotosClient.unpackThumbnails(data) == [1: Data([1, 2])])
    }
}

}
