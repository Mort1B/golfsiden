// Built in memory from the current real decoder; never alters the app bundle.
import { decodeListing } from '../../../frontend/src/api/matchPlay/decoders.ts'
window.decodeListingProbe = decodeListing
