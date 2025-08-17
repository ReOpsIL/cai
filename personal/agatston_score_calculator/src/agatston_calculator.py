import numpy as np
from scipy.ndimage import label

AGATSTON_THRESHOLD = 130 # Hounsfield Units

def calculate_agatston_score(hu_volume, pixel_spacing, slice_thickness):
    """Calculates the Agatston score for a given HU volume.

    Args:
        hu_volume (np.ndarray): 3D numpy array of Hounsfield Units.
        pixel_spacing (list): A list or tuple of [row_spacing, col_spacing] in mm.
        slice_thickness (float): Thickness of each slice in mm.

    Returns:
        float: The calculated Agatston score.
    """
    score = 0.0
    voxel_area = pixel_spacing[0] * pixel_spacing[1]

    # Iterate through each slice
    for i in range(hu_volume.shape[0]):
        slice_hu = hu_volume[i, :, :]

        # Threshold the image to find calcifications (HU > 130)
        binary_calcifications = (slice_hu > AGATSTON_THRESHOLD).astype(int)

        # Label connected components (calcification regions)
        labeled_array, num_features = label(binary_calcifications)

        for region_id in range(1, num_features + 1):
            # Extract the current calcification region
            current_region = (labeled_array == region_id)

            # Calculate the area of the region
            region_area = np.sum(current_region) * voxel_area

            # Find the maximum HU value within this region
            max_hu_in_region = np.max(slice_hu[current_region])

            # Assign Agatston factor based on max HU
            if max_hu_in_region >= 400:
                agatston_factor = 4
            elif max_hu_in_region >= 300:
                agatston_factor = 3
            elif max_hu_in_region >= 200:
                agatston_factor = 2
            elif max_hu_in_region >= AGATSTON_THRESHOLD:
                agatston_factor = 1
            else:
                agatston_factor = 0 # Should not happen if thresholding is correct

            score += region_area * agatston_factor

    return score
