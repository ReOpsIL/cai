import os
import sys
from .dicom_utils import load_dicom_series, get_pixels_hu
from .agatston_calculator import calculate_agatston_score

def main(dicom_directory):
    if not os.path.isdir(dicom_directory):
        print(f"Error: Directory '{dicom_directory}' not found.")
        sys.exit(1)

    print(f"Loading DICOM series from: {dicom_directory}")
    dicom_slices = load_dicom_series(dicom_directory)

    if not dicom_slices:
        print("No DICOM files found in the specified directory.")
        sys.exit(1)

    print(f"Converting pixel data to Hounsfield Units...")
    hu_volume = get_pixels_hu(dicom_slices)

    # Get pixel spacing and slice thickness from the first slice
    pixel_spacing = dicom_slices[0].PixelSpacing
    slice_thickness = dicom_slices[0].SliceThickness

    print(f"Calculating Agatston score...")
    agatston_score = calculate_agatston_score(hu_volume, pixel_spacing, slice_thickness)

    print(f"\nAgatston Score: {agatston_score:.2f}")

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python main.py <path_to_dicom_directory>")
        sys.exit(1)
    
    dicom_path = sys.argv[1]
    main(dicom_path)
