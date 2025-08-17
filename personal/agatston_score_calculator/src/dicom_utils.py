import pydicom
import numpy as np
import os

def load_dicom_series(directory):
    """Loads a DICOM series from a given directory."""
    slices = []
    for filename in os.listdir(directory):
        if filename.endswith('.dcm') or filename.endswith('.DCM'):
            slices.append(pydicom.dcmread(os.path.join(directory, filename)))

    slices.sort(key=lambda x: float(x.ImagePositionPatient[2]) if hasattr(x, 'ImagePositionPatient') else float(x.InstanceNumber))

    if len(slices) > 1:
        try:
            slice_thickness = np.abs(slices[0].ImagePositionPatient[2] - slices[1].ImagePositionPatient[2])
        except AttributeError:
            slice_thickness = np.abs(slices[0].SliceLocation - slices[1].SliceLocation)
    else:
        slice_thickness = slices[0].SliceThickness if hasattr(slices[0], 'SliceThickness') else 1.0 # Default or handle single slice case

    for s in slices:
        s.SliceThickness = slice_thickness

    return slices

def get_pixels_hu(slices):
    """Converts DICOM pixel data to Hounsfield Units (HU)."""
    image = np.stack([s.pixel_array for s in slices])
    image = image.astype(np.int16)

    # Set outside-of-scan pixels to 0
    # The intercept is usually -1024, so air is approximately 0
    image[image == -2000] = 0

    # Convert to Hounsfield units (HU)
    for slice_number in range(len(slices)):
        intercept = slices[slice_number].RescaleIntercept if 'RescaleIntercept' in slices[slice_number] else 0
        slope = slices[slice_number].RescaleSlope if 'RescaleSlope' in slices[slice_number] else 1

        if slope != 1:
            image[slice_number] = slope * image[slice_number].astype(np.float64)
            image[slice_number] = image[slice_number].astype(np.int16)

        image[slice_number] += np.int16(intercept)

    return np.array(image, dtype=np.int16)
