import { CREATE_CHANNEL_IN_PROGRESS, CHANNEL_CREATED, CHANNEL_NOT_CREATED, CHANNEL_RESET } from './actionTypes'

const initialState = {
    isLoading: false,
    isSuccess: false,
    error: '',
    data: '',
};

export default (state = initialState, action) => {
    switch (action.type) {
        case CREATE_CHANNEL_IN_PROGRESS:
            return { ...state, isLoading: true, error: '', isSuccess: false };
        case CHANNEL_CREATED:
            return { ...state, data: action, isLoading: false, error: '', isSuccess: true };
        case CHANNEL_NOT_CREATED:
            return { ...state, error: action.errorMessage, isLoading: false, isSuccess: false };
        case CHANNEL_RESET:
            return { error: '', isLoading: false, data: action.data, isSuccess: false };
        default:
            return state;
    }
};